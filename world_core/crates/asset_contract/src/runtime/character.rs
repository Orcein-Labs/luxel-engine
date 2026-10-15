//! A deterministic CPU oracle for one selected, embedded glTF skinned mesh.
//!
//! `DecodedCharacter` is an evaluation input, not an asset-preparation receipt
//! or production-readiness verdict. Mesh positions and bounds are in the
//! selected mesh node's local space. Socket matrices map node-local points into
//! the selected scene's source coordinate space. All matrices are column-major.

use std::collections::BTreeSet;
use std::fmt;

use crate::{AssetContractError, Bounds3, inspect_glb};
use serde_json::Value;

use super::{determinant4, parent_map, read_component, read_weight};

pub type CharacterPosition3 = [f64; 3];
pub type CharacterMatrix4 = [f64; 16];
type Vec3 = CharacterPosition3;
type Quat = [f64; 4];
type Mat4 = CharacterMatrix4;
const IDENTITY: Mat4 = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
];
const WEIGHT_TOLERANCE: f64 = 1e-4;

/// Execution-profile limits, independent of asset preparation acceptance.
pub const MAX_CPU_CHARACTER_NODES: usize = 4096;
pub const MAX_CPU_CHARACTER_HIERARCHY_DEPTH: usize = 128;

/// Selects one active skinned mesh-node instance, named clip, and socket set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterSelection {
    pub mesh_node_name: String,
    pub clip_name: String,
    pub socket_node_names: Vec<String>,
}

impl CharacterSelection {
    pub fn new(
        mesh_node_name: impl Into<String>,
        clip_name: impl Into<String>,
        socket_node_names: Vec<String>,
    ) -> Self {
        Self {
            mesh_node_name: mesh_node_name.into(),
            clip_name: clip_name.into(),
            socket_node_names,
        }
    }
}

/// A bounded failure from parsing or evaluating the selected character data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CharacterEvaluationError {
    InvalidGlb(String),
    UnsupportedRequiredExtension(String),
    ExternalBuffer,
    SparseAccessor(String),
    UnsupportedFeature(String),
    UnsupportedCubicSpline,
    UnsupportedMorphTargets,
    UnsupportedMorphChannel,
    UnsupportedInfluenceSet,
    InvalidSelection(String),
    AmbiguousSelection(String),
    InvalidHierarchy(String),
    InvalidAccessor(String),
    InvalidJointIndex(String),
    InvalidWeights(String),
    InvalidTransform(String),
    InvalidAnimation(String),
    NonFiniteTime,
}

impl fmt::Display for CharacterEvaluationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidGlb(detail) => write!(f, "invalid GLB: {detail}"),
            Self::UnsupportedRequiredExtension(name) => {
                write!(f, "unsupported required glTF extension {name}")
            }
            Self::ExternalBuffer => f.write_str("external glTF buffers are unsupported"),
            Self::SparseAccessor(name) => write!(f, "sparse accessor is unsupported: {name}"),
            Self::UnsupportedFeature(name) => write!(f, "unsupported character feature: {name}"),
            Self::UnsupportedCubicSpline => f.write_str("CUBICSPLINE animation is unsupported"),
            Self::UnsupportedMorphTargets => f.write_str("morph targets are unsupported"),
            Self::UnsupportedMorphChannel => {
                f.write_str("morph animation channels are unsupported")
            }
            Self::UnsupportedInfluenceSet => f.write_str("JOINTS_1 and WEIGHTS_1 are unsupported"),
            Self::InvalidSelection(detail) => write!(f, "invalid character selection: {detail}"),
            Self::AmbiguousSelection(detail) => {
                write!(f, "ambiguous character selection: {detail}")
            }
            Self::InvalidHierarchy(detail) => write!(f, "invalid character hierarchy: {detail}"),
            Self::InvalidAccessor(detail) => write!(f, "invalid character accessor: {detail}"),
            Self::InvalidJointIndex(detail) => write!(f, "invalid skin joint index: {detail}"),
            Self::InvalidWeights(detail) => write!(f, "invalid skin weights: {detail}"),
            Self::InvalidTransform(detail) => write!(f, "invalid character transform: {detail}"),
            Self::InvalidAnimation(detail) => write!(f, "invalid character animation: {detail}"),
            Self::NonFiniteTime => f.write_str("sample time must be finite"),
        }
    }
}

impl std::error::Error for CharacterEvaluationError {}

/// Checked source data for CPU evaluation. It does not imply production readiness.
#[derive(Debug, Clone, PartialEq)]
pub struct DecodedCharacter {
    source_sha256: String,
    selection: CharacterSelection,
    mesh_node: usize,
    nodes: Vec<NodeTransform>,
    parents: Vec<Option<usize>>,
    scene_roots: Vec<usize>,
    active_nodes: BTreeSet<usize>,
    joint_nodes: Vec<usize>,
    inverse_bind_matrices: Vec<Mat4>,
    positions: Vec<Vec3>,
    joint_indices: Vec<[usize; 4]>,
    weights: Vec<[f64; 4]>,
    channels: Vec<AnimationChannel>,
    sockets: Vec<(String, usize)>,
    clip_start: f64,
    clip_end: f64,
}

impl DecodedCharacter {
    pub fn source_sha256(&self) -> &str {
        &self.source_sha256
    }

    pub fn selection(&self) -> &CharacterSelection {
        &self.selection
    }

    pub fn mesh_node_index(&self) -> usize {
        self.mesh_node
    }

    pub fn joint_node_indices(&self) -> &[usize] {
        &self.joint_nodes
    }

    /// Stateless sample; time is clamped to the selected clip's key-time range.
    pub fn evaluate(
        &self,
        time_seconds: f64,
    ) -> Result<CpuCharacterPose, CharacterEvaluationError> {
        if !time_seconds.is_finite() {
            return Err(CharacterEvaluationError::NonFiniteTime);
        }
        let sample_time = time_seconds.clamp(self.clip_start, self.clip_end);
        let mut transforms = self.nodes.clone();
        for channel in &self.channels {
            let value = channel.sample(sample_time);
            match channel.path {
                TransformPath::Translation => {
                    transforms[channel.node].translation = [value[0], value[1], value[2]];
                }
                TransformPath::Rotation => {
                    transforms[channel.node].rotation = [value[0], value[1], value[2], value[3]];
                }
                TransformPath::Scale => {
                    transforms[channel.node].scale = [value[0], value[1], value[2]];
                }
            }
        }
        let globals = compute_globals(
            &transforms,
            &self.parents,
            &self.scene_roots,
            &self.active_nodes,
        )?;
        let mesh_global = globals[self.mesh_node];
        let mesh_inverse = inverse(mesh_global).ok_or_else(|| {
            CharacterEvaluationError::InvalidTransform("selected mesh node is singular".into())
        })?;
        let mut skin_matrices = Vec::with_capacity(self.joint_nodes.len());
        for (joint, inverse_bind) in self.joint_nodes.iter().zip(&self.inverse_bind_matrices) {
            let joint_global = globals[*joint];
            if determinant4(joint_global).abs() < 1e-12 {
                return Err(CharacterEvaluationError::InvalidTransform(format!(
                    "joint node {joint} is singular"
                )));
            }
            skin_matrices.push(multiply(
                mesh_inverse,
                multiply(joint_global, *inverse_bind),
            ));
        }
        let mut positions = Vec::with_capacity(self.positions.len());
        for ((position, joints), weights) in self
            .positions
            .iter()
            .zip(&self.joint_indices)
            .zip(&self.weights)
        {
            let mut posed = [0.0; 3];
            for influence in 0..4 {
                let weight = weights[influence];
                if weight == 0.0 {
                    continue;
                }
                let point = transform_point(skin_matrices[joints[influence]], *position);
                for axis in 0..3 {
                    posed[axis] += weight * point[axis];
                }
            }
            if !posed.iter().all(|value| value.is_finite()) {
                return Err(CharacterEvaluationError::InvalidTransform(
                    "skinning produced a non-finite vertex".into(),
                ));
            }
            positions.push(posed);
        }
        let bounds_mesh_local = bounds(&positions);
        let mut sockets = Vec::with_capacity(self.sockets.len());
        for (name, node) in &self.sockets {
            let matrix = globals[*node];
            if determinant4(matrix).abs() < 1e-12 {
                return Err(CharacterEvaluationError::InvalidTransform(format!(
                    "socket node {node} is singular"
                )));
            }
            sockets.push(CharacterSocketTransform {
                name: name.clone(),
                node_index: *node,
                scene_from_node: matrix,
            });
        }
        Ok(CpuCharacterPose {
            requested_time_seconds: time_seconds,
            sample_time_seconds: sample_time,
            positions_mesh_local: positions,
            sockets,
            bounds_mesh_local,
        })
    }
}

/// Evaluated geometry and scene-space sockets, never a production readiness verdict.
#[derive(Debug, Clone, PartialEq)]
pub struct CpuCharacterPose {
    pub requested_time_seconds: f64,
    pub sample_time_seconds: f64,
    pub positions_mesh_local: Vec<CharacterPosition3>,
    pub sockets: Vec<CharacterSocketTransform>,
    pub bounds_mesh_local: Bounds3,
}

/// Column-major matrix mapping socket-local coordinates into the selected scene.
#[derive(Debug, Clone, PartialEq)]
pub struct CharacterSocketTransform {
    pub name: String,
    pub node_index: usize,
    pub scene_from_node: CharacterMatrix4,
}

/// Decode and validate the bounded CPU profile from embedded GLB bytes.
///
/// This function returns a numerical evaluation input only. Callers must still
/// use `prepare_asset` for application-specific production acceptance.
pub fn decode_character_glb(
    bytes: &[u8],
    selection: &CharacterSelection,
) -> Result<DecodedCharacter, CharacterEvaluationError> {
    let report = inspect_glb(bytes).map_err(map_glb_error)?;
    if let Some(extension) = report.gltf.extensions_required.first() {
        return Err(CharacterEvaluationError::UnsupportedRequiredExtension(
            extension.clone(),
        ));
    }
    let (document, binary, _) = crate::parse_glb(bytes).map_err(map_glb_error)?;
    crate::validate_glb_buffers(&document, binary.len()).map_err(map_glb_error)?;
    let nodes = array(&document, "nodes", "nodes")?;
    if nodes.len() > MAX_CPU_CHARACTER_NODES {
        return Err(CharacterEvaluationError::UnsupportedFeature(format!(
            "CPU character node limit is {MAX_CPU_CHARACTER_NODES}"
        )));
    }
    let skins = array(&document, "skins", "skins")?;
    let meshes = array(&document, "meshes", "meshes")?;
    if skins.len() != 1 {
        return Err(CharacterEvaluationError::UnsupportedFeature(
            "the selected character profile requires exactly one skin".into(),
        ));
    }
    for (index, node) in nodes.iter().enumerate() {
        if node.get("skin").is_some()
            && (index_field(node, "skin") != Some(0) || index_field(node, "mesh").is_none())
        {
            return Err(CharacterEvaluationError::InvalidHierarchy(format!(
                "node {index} has an invalid skin or no bound mesh"
            )));
        }
    }
    validate_child_lists(nodes)?;
    let parents = parent_map(nodes).map_err(CharacterEvaluationError::InvalidHierarchy)?;
    validate_hierarchy_depth(&parents)?;
    let scene_roots = scene_roots(&document, nodes, &parents)?;
    let active_nodes = crate::active_node_indices(&document, nodes)
        .map_err(|error| CharacterEvaluationError::InvalidHierarchy(error.to_string()))?;
    let mesh_node = unique_node(nodes, &selection.mesh_node_name)?;
    if !active_nodes.contains(&mesh_node) {
        return Err(CharacterEvaluationError::InvalidSelection(
            "selected mesh node is outside the active scene".into(),
        ));
    }
    let node = &nodes[mesh_node];
    let mesh_index = index_field(node, "mesh").ok_or_else(|| {
        CharacterEvaluationError::InvalidSelection("selected node does not reference a mesh".into())
    })?;
    if index_field(node, "skin") != Some(0) {
        return Err(CharacterEvaluationError::InvalidSelection(
            "selected mesh node must reference the single skin".into(),
        ));
    }
    let mesh = meshes.get(mesh_index).ok_or_else(|| {
        CharacterEvaluationError::InvalidSelection("selected mesh index is out of range".into())
    })?;
    if morph_weights(node, "selected node")? || morph_weights(mesh, "selected mesh")? {
        return Err(CharacterEvaluationError::UnsupportedMorphTargets);
    }
    let primitives = mesh
        .get("primitives")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            CharacterEvaluationError::InvalidSelection("selected mesh has no primitives".into())
        })?;
    if primitives.is_empty() {
        return Err(CharacterEvaluationError::InvalidSelection(
            "selected mesh has no primitives".into(),
        ));
    }
    let skin = &skins[0];
    let joint_values = skin
        .get("joints")
        .and_then(Value::as_array)
        .filter(|joints| !joints.is_empty())
        .ok_or_else(|| {
            CharacterEvaluationError::InvalidHierarchy("skin joints are missing".into())
        })?;
    let mut joint_nodes = Vec::with_capacity(joint_values.len());
    let mut joint_set = BTreeSet::new();
    for value in joint_values {
        let joint = value
            .as_u64()
            .and_then(|index| usize::try_from(index).ok())
            .filter(|index| *index < nodes.len())
            .ok_or_else(|| {
                CharacterEvaluationError::InvalidHierarchy("skin joint index is invalid".into())
            })?;
        if !joint_set.insert(joint) || !active_nodes.contains(&joint) {
            return Err(CharacterEvaluationError::InvalidHierarchy(
                "skin joints must be unique and active in the selected scene".into(),
            ));
        }
        joint_nodes.push(joint);
    }
    if skin.get("skeleton").is_some() {
        let skeleton = index_field(skin, "skeleton").ok_or_else(|| {
            CharacterEvaluationError::InvalidHierarchy("skin skeleton index is invalid".into())
        })?;
        if skeleton >= nodes.len()
            || joint_nodes
                .iter()
                .any(|joint| !is_descendant(*joint, skeleton, &parents))
        {
            return Err(CharacterEvaluationError::InvalidHierarchy(
                "skin skeleton must contain every joint".into(),
            ));
        }
    }
    let inverse_bind_matrices = decode_inverse_binds(&document, binary, skin, joint_nodes.len())?;
    let mut positions = Vec::new();
    let mut joint_indices = Vec::new();
    let mut weights = Vec::new();
    for (primitive_index, primitive) in primitives.iter().enumerate() {
        if let Some(targets) = primitive.get("targets") {
            let targets = targets.as_array().ok_or_else(|| {
                CharacterEvaluationError::InvalidAccessor(format!(
                    "primitive {primitive_index} targets must be an array"
                ))
            })?;
            if !targets.is_empty() {
                return Err(CharacterEvaluationError::UnsupportedMorphTargets);
            }
        }
        let attributes = primitive
            .get("attributes")
            .and_then(Value::as_object)
            .ok_or_else(|| {
                CharacterEvaluationError::InvalidAccessor(format!(
                    "primitive {primitive_index} has no attribute map"
                ))
            })?;
        if attributes.contains_key("JOINTS_1") || attributes.contains_key("WEIGHTS_1") {
            return Err(CharacterEvaluationError::UnsupportedInfluenceSet);
        }
        let position_index = attribute_index(attributes, "POSITION", primitive_index)?;
        let joint_index = attribute_index(attributes, "JOINTS_0", primitive_index)?;
        let weight_index = attribute_index(attributes, "WEIGHTS_0", primitive_index)?;
        let position_info = accessor(&document, binary, position_index)?;
        let joint_info = accessor(&document, binary, joint_index)?;
        let weight_info = accessor(&document, binary, weight_index)?;
        if position_info.accessor_type != "VEC3"
            || position_info.component_type != 5126
            || position_info.normalized
        {
            return Err(CharacterEvaluationError::InvalidAccessor(format!(
                "primitive {primitive_index} POSITION must be FLOAT VEC3"
            )));
        }
        if joint_info.accessor_type != "VEC4"
            || !matches!(joint_info.component_type, 5121 | 5123)
            || joint_info.normalized
        {
            return Err(CharacterEvaluationError::InvalidAccessor(format!(
                "primitive {primitive_index} JOINTS_0 must be unnormalized unsigned VEC4"
            )));
        }
        let valid_weights = weight_info.accessor_type == "VEC4"
            && (weight_info.component_type == 5126 && !weight_info.normalized
                || matches!(weight_info.component_type, 5121 | 5123) && weight_info.normalized);
        if !valid_weights {
            return Err(CharacterEvaluationError::InvalidAccessor(format!(
                "primitive {primitive_index} WEIGHTS_0 must be FLOAT or normalized unsigned VEC4"
            )));
        }
        if position_info.count != joint_info.count || position_info.count != weight_info.count {
            return Err(CharacterEvaluationError::InvalidAccessor(format!(
                "primitive {primitive_index} skin and position counts differ"
            )));
        }
        for vertex in 0..position_info.count {
            let mut position = [0.0; 3];
            for (component, value) in position.iter_mut().enumerate() {
                *value =
                    read_component(binary, &position_info, vertex, component).ok_or_else(|| {
                        CharacterEvaluationError::InvalidAccessor("POSITION is unreadable".into())
                    })?;
            }
            if !position.iter().all(|value| value.is_finite()) {
                return Err(CharacterEvaluationError::InvalidAccessor(
                    "POSITION contains a non-finite value".into(),
                ));
            }
            let mut vertex_joints = [0; 4];
            let mut vertex_weights = [0.0; 4];
            let mut total_weight = 0.0;
            for influence in 0..4 {
                vertex_joints[influence] = read_component(binary, &joint_info, vertex, influence)
                    .map(|value| value as usize)
                    .filter(|joint| *joint < joint_nodes.len())
                    .ok_or_else(|| {
                        CharacterEvaluationError::InvalidJointIndex(format!(
                            "primitive {primitive_index} vertex {vertex} has an invalid joint index"
                        ))
                    })?;
                vertex_weights[influence] = read_weight(binary, &weight_info, vertex, influence)
                    .filter(|weight| weight.is_finite() && *weight >= 0.0)
                    .ok_or_else(|| {
                        CharacterEvaluationError::InvalidWeights(format!(
                            "primitive {primitive_index} vertex {vertex} has an invalid weight"
                        ))
                    })?;
                total_weight += vertex_weights[influence];
            }
            if total_weight <= 0.0 || (total_weight - 1.0).abs() > WEIGHT_TOLERANCE {
                return Err(CharacterEvaluationError::InvalidWeights(format!(
                    "primitive {primitive_index} vertex {vertex} weights must sum to one"
                )));
            }
            positions.push(position);
            joint_indices.push(vertex_joints);
            weights.push(vertex_weights);
        }
    }
    if positions.is_empty() {
        return Err(CharacterEvaluationError::InvalidSelection(
            "selected mesh has no vertices".into(),
        ));
    }
    let mut socket_names = BTreeSet::new();
    let mut sockets = Vec::with_capacity(selection.socket_node_names.len());
    for name in &selection.socket_node_names {
        if name.trim().is_empty() || !socket_names.insert(name.clone()) {
            return Err(CharacterEvaluationError::InvalidSelection(
                "socket names must be non-empty and unique".into(),
            ));
        }
        let socket = unique_node(nodes, name)?;
        if !active_nodes.contains(&socket) {
            return Err(CharacterEvaluationError::InvalidSelection(format!(
                "socket node {name} is outside the active scene"
            )));
        }
        sockets.push((name.clone(), socket));
    }
    let node_transforms = nodes
        .iter()
        .enumerate()
        .map(|(index, node)| decode_node_transform(node, index))
        .collect::<Result<Vec<_>, _>>()?;
    let (channels, clip_start, clip_end) = decode_animation(
        &document,
        binary,
        nodes,
        &active_nodes,
        &selection.clip_name,
        &node_transforms,
    )?;
    let decoded = DecodedCharacter {
        source_sha256: report.identity.source_sha256,
        selection: selection.clone(),
        mesh_node,
        nodes: node_transforms,
        parents,
        scene_roots,
        active_nodes,
        joint_nodes,
        inverse_bind_matrices,
        positions,
        joint_indices,
        weights,
        channels,
        sockets,
        clip_start,
        clip_end,
    };
    let rest = decoded.evaluate(clip_start)?;
    if rest.positions_mesh_local.len() != decoded.positions.len() {
        return Err(CharacterEvaluationError::InvalidTransform(
            "rest-pose evaluation did not preserve vertex count".into(),
        ));
    }
    Ok(decoded)
}

#[derive(Debug, Clone, PartialEq)]
struct NodeTransform {
    matrix: Option<Mat4>,
    translation: Vec3,
    rotation: Quat,
    scale: Vec3,
}

impl NodeTransform {
    fn matrix(&self) -> Mat4 {
        self.matrix
            .unwrap_or_else(|| trs_matrix(self.translation, self.rotation, self.scale))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum TransformPath {
    Translation,
    Rotation,
    Scale,
}

#[derive(Debug, Clone, PartialEq)]
struct AnimationChannel {
    node: usize,
    path: TransformPath,
    interpolation: Interpolation,
    times: Vec<f64>,
    values: Vec<[f64; 4]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Interpolation {
    Step,
    Linear,
}

impl AnimationChannel {
    fn sample(&self, requested_time: f64) -> [f64; 4] {
        let time = requested_time.clamp(self.times[0], *self.times.last().unwrap());
        let upper = self
            .times
            .iter()
            .position(|key| *key > time)
            .unwrap_or(self.times.len());
        let left = upper.saturating_sub(1);
        if upper == self.times.len() || self.interpolation == Interpolation::Step {
            return self.values[left];
        }
        let right = upper;
        let amount = (time - self.times[left]) / (self.times[right] - self.times[left]);
        if self.path == TransformPath::Rotation {
            slerp(self.values[left], self.values[right], amount)
        } else {
            std::array::from_fn(|index| {
                self.values[left][index] * (1.0 - amount) + self.values[right][index] * amount
            })
        }
    }
}

fn decode_animation(
    document: &Value,
    binary: &[u8],
    nodes: &[Value],
    active_nodes: &BTreeSet<usize>,
    clip_name: &str,
    node_transforms: &[NodeTransform],
) -> Result<(Vec<AnimationChannel>, f64, f64), CharacterEvaluationError> {
    let animations = array(document, "animations", "animations")?;
    let matches = animations
        .iter()
        .enumerate()
        .filter(|(_, animation)| animation.get("name").and_then(Value::as_str) == Some(clip_name))
        .collect::<Vec<_>>();
    if matches.is_empty() {
        return Err(CharacterEvaluationError::InvalidSelection(format!(
            "named clip {clip_name} is missing"
        )));
    }
    if matches.len() != 1 {
        return Err(CharacterEvaluationError::AmbiguousSelection(format!(
            "clip name {clip_name} occurs more than once"
        )));
    }
    let animation = matches[0].1;
    let samplers = animation
        .get("samplers")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            CharacterEvaluationError::InvalidAnimation("samplers must be an array".into())
        })?;
    let channels = animation
        .get("channels")
        .and_then(Value::as_array)
        .filter(|channels| !channels.is_empty())
        .ok_or_else(|| {
            CharacterEvaluationError::InvalidAnimation("clip channels are missing".into())
        })?;
    let mut output = Vec::with_capacity(channels.len());
    let mut targets = BTreeSet::new();
    let mut clip_start = f64::INFINITY;
    let mut clip_end = f64::NEG_INFINITY;
    for (channel_index, channel) in channels.iter().enumerate() {
        let sampler_index = index_field(channel, "sampler").ok_or_else(|| {
            CharacterEvaluationError::InvalidAnimation(format!(
                "channel {channel_index} has no sampler index"
            ))
        })?;
        let sampler = samplers.get(sampler_index).ok_or_else(|| {
            CharacterEvaluationError::InvalidAnimation(format!(
                "channel {channel_index} sampler is out of range"
            ))
        })?;
        let target = channel.get("target").ok_or_else(|| {
            CharacterEvaluationError::InvalidAnimation(format!(
                "channel {channel_index} target is missing"
            ))
        })?;
        let node = index_field(target, "node")
            .filter(|index| *index < nodes.len())
            .ok_or_else(|| {
                CharacterEvaluationError::InvalidAnimation(format!(
                    "channel {channel_index} target node is invalid"
                ))
            })?;
        if !active_nodes.contains(&node) {
            return Err(CharacterEvaluationError::InvalidAnimation(format!(
                "channel {channel_index} targets a node outside the active scene"
            )));
        }
        let path = match target.get("path").and_then(Value::as_str) {
            Some("translation") => TransformPath::Translation,
            Some("rotation") => TransformPath::Rotation,
            Some("scale") => TransformPath::Scale,
            Some("weights") => return Err(CharacterEvaluationError::UnsupportedMorphChannel),
            Some(other) => {
                return Err(CharacterEvaluationError::UnsupportedFeature(format!(
                    "animation target path {other}"
                )));
            }
            None => {
                return Err(CharacterEvaluationError::InvalidAnimation(
                    "target path is missing".into(),
                ));
            }
        };
        if node_transforms[node].matrix.is_some() {
            return Err(CharacterEvaluationError::InvalidAnimation(format!(
                "channel {channel_index} animates a matrix-authored node"
            )));
        }
        if !targets.insert((node, path)) {
            return Err(CharacterEvaluationError::InvalidAnimation(
                "clip repeats a node/path target".into(),
            ));
        }
        let interpolation = match sampler
            .get("interpolation")
            .and_then(Value::as_str)
            .unwrap_or("LINEAR")
        {
            "LINEAR" => Interpolation::Linear,
            "STEP" => Interpolation::Step,
            "CUBICSPLINE" => return Err(CharacterEvaluationError::UnsupportedCubicSpline),
            other => {
                return Err(CharacterEvaluationError::UnsupportedFeature(format!(
                    "animation interpolation {other}"
                )));
            }
        };
        let input_index = index_field(sampler, "input").ok_or_else(|| {
            CharacterEvaluationError::InvalidAnimation("sampler input accessor is missing".into())
        })?;
        let output_index = index_field(sampler, "output").ok_or_else(|| {
            CharacterEvaluationError::InvalidAnimation("sampler output accessor is missing".into())
        })?;
        let input = accessor(document, binary, input_index)?;
        let values = accessor(document, binary, output_index)?;
        if input.accessor_type != "SCALAR"
            || input.component_type != 5126
            || input.normalized
            || input.count == 0
        {
            return Err(CharacterEvaluationError::InvalidAnimation(
                "sampler times must be a non-empty FLOAT SCALAR accessor".into(),
            ));
        }
        let component_count = if path == TransformPath::Rotation {
            4
        } else {
            3
        };
        let expected_type = if component_count == 4 { "VEC4" } else { "VEC3" };
        if values.accessor_type != expected_type
            || values.component_type != 5126
            || values.normalized
            || values.count != input.count
        {
            return Err(CharacterEvaluationError::InvalidAnimation(
                "sampler output type/count does not match its transform path and keys".into(),
            ));
        }
        let times = (0..input.count)
            .map(|key| read_component(binary, &input, key, 0))
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| {
                CharacterEvaluationError::InvalidAnimation("sampler times are unreadable".into())
            })?;
        if times.iter().any(|time| !time.is_finite() || *time < 0.0)
            || times.windows(2).any(|pair| pair[1] <= pair[0])
        {
            return Err(CharacterEvaluationError::InvalidAnimation(
                "key times must be finite, non-negative, and strictly increasing".into(),
            ));
        }
        let mut samples = Vec::with_capacity(values.count);
        for key in 0..values.count {
            let mut sample = [0.0; 4];
            for (component, slot) in sample.iter_mut().enumerate().take(component_count) {
                *slot =
                    read_component(binary, &values, key, component).ok_or_else(|| {
                        CharacterEvaluationError::InvalidAnimation(
                            "sampler output is unreadable".into(),
                        )
                    })?;
            }
            if !sample
                .iter()
                .take(component_count)
                .all(|value| value.is_finite())
            {
                return Err(CharacterEvaluationError::InvalidAnimation(
                    "sampler output contains a non-finite value".into(),
                ));
            }
            if path == TransformPath::Rotation {
                sample = normalize_quat(sample).ok_or_else(|| {
                    CharacterEvaluationError::InvalidAnimation(
                        "rotation key quaternion is zero or non-finite".into(),
                    )
                })?;
            }
            samples.push(sample);
        }
        clip_start = clip_start.min(times[0]);
        clip_end = clip_end.max(*times.last().unwrap());
        output.push(AnimationChannel {
            node,
            path,
            interpolation,
            times,
            values: samples,
        });
    }
    Ok((output, clip_start, clip_end))
}

fn decode_inverse_binds(
    document: &Value,
    binary: &[u8],
    skin: &Value,
    joint_count: usize,
) -> Result<Vec<Mat4>, CharacterEvaluationError> {
    let Some(value) = skin.get("inverseBindMatrices") else {
        return Ok(vec![IDENTITY; joint_count]);
    };
    let index = value
        .as_u64()
        .and_then(|index| usize::try_from(index).ok())
        .ok_or_else(|| {
            CharacterEvaluationError::InvalidAccessor(
                "inverse bind matrix accessor index is invalid".into(),
            )
        })?;
    let info = accessor(document, binary, index)?;
    if info.accessor_type != "MAT4"
        || info.component_type != 5126
        || info.normalized
        || info.count != joint_count
    {
        return Err(CharacterEvaluationError::InvalidAccessor(
            "inverse bind matrices must be FLOAT MAT4 with one per joint".into(),
        ));
    }
    let mut matrices = Vec::with_capacity(joint_count);
    for matrix_index in 0..joint_count {
        let mut matrix = [0.0; 16];
        for (component, value) in matrix.iter_mut().enumerate() {
            *value = read_component(binary, &info, matrix_index, component).ok_or_else(|| {
                CharacterEvaluationError::InvalidAccessor(
                    "inverse bind matrix is unreadable".into(),
                )
            })?;
        }
        if !matrix.iter().all(|value| value.is_finite())
            || matrix[3].abs() > 1e-12
            || matrix[7].abs() > 1e-12
            || matrix[11].abs() > 1e-12
            || (matrix[15] - 1.0).abs() > 1e-12
            || determinant4(matrix).abs() < 1e-12
        {
            return Err(CharacterEvaluationError::InvalidTransform(format!(
                "inverse bind matrix {matrix_index} is non-finite or singular"
            )));
        }
        matrices.push(matrix);
    }
    Ok(matrices)
}

fn decode_node_transform(
    node: &Value,
    index: usize,
) -> Result<NodeTransform, CharacterEvaluationError> {
    if node.get("matrix").is_some()
        && (node.get("translation").is_some()
            || node.get("rotation").is_some()
            || node.get("scale").is_some())
    {
        return Err(CharacterEvaluationError::InvalidTransform(format!(
            "node {index} specifies both matrix and TRS"
        )));
    }
    if let Some(value) = node.get("matrix") {
        let values = value
            .as_array()
            .filter(|values| values.len() == 16)
            .ok_or_else(|| {
                CharacterEvaluationError::InvalidTransform(format!(
                    "node {index} matrix must contain 16 values"
                ))
            })?;
        let matrix = values
            .iter()
            .map(|value| value.as_f64().filter(|value| value.is_finite()))
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| {
                CharacterEvaluationError::InvalidTransform(format!(
                    "node {index} matrix is non-finite"
                ))
            })?;
        let matrix: Mat4 = matrix.try_into().unwrap();
        if matrix[3].abs() > 1e-12
            || matrix[7].abs() > 1e-12
            || matrix[11].abs() > 1e-12
            || (matrix[15] - 1.0).abs() > 1e-12
        {
            return Err(CharacterEvaluationError::InvalidTransform(format!(
                "node {index} matrix must be affine"
            )));
        }
        return Ok(NodeTransform {
            matrix: Some(matrix),
            translation: [0.0; 3],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0; 3],
        });
    }
    let translation = vector3(node.get("translation"), [0.0; 3], index, "translation")?;
    let scale = vector3(node.get("scale"), [1.0; 3], index, "scale")?;
    let rotation = match node.get("rotation") {
        None => [0.0, 0.0, 0.0, 1.0],
        Some(value) => {
            let values = value
                .as_array()
                .filter(|values| values.len() == 4)
                .ok_or_else(|| {
                    CharacterEvaluationError::InvalidTransform(format!(
                        "node {index} rotation must be XYZW"
                    ))
                })?;
            let values = values
                .iter()
                .map(|value| value.as_f64().filter(|value| value.is_finite()))
                .collect::<Option<Vec<_>>>()
                .ok_or_else(|| {
                    CharacterEvaluationError::InvalidTransform(format!(
                        "node {index} rotation is non-finite"
                    ))
                })?;
            normalize_quat(values.try_into().unwrap()).ok_or_else(|| {
                CharacterEvaluationError::InvalidTransform(format!("node {index} rotation is zero"))
            })?
        }
    };
    Ok(NodeTransform {
        matrix: None,
        translation,
        rotation,
        scale,
    })
}

fn vector3(
    value: Option<&Value>,
    default: Vec3,
    node: usize,
    field: &str,
) -> Result<Vec3, CharacterEvaluationError> {
    let Some(value) = value else {
        return Ok(default);
    };
    let values = value
        .as_array()
        .filter(|values| values.len() == 3)
        .ok_or_else(|| {
            CharacterEvaluationError::InvalidTransform(format!(
                "node {node} {field} must contain three values"
            ))
        })?;
    let values = values
        .iter()
        .map(|value| value.as_f64().filter(|value| value.is_finite()))
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| {
            CharacterEvaluationError::InvalidTransform(format!("node {node} {field} is non-finite"))
        })?;
    Ok(values.try_into().unwrap())
}

fn accessor<'a>(
    document: &'a Value,
    binary: &[u8],
    index: usize,
) -> Result<crate::AccessorInfo<'a>, CharacterEvaluationError> {
    let accessors = array(document, "accessors", "accessors")?;
    let value = accessors.get(index).ok_or_else(|| {
        CharacterEvaluationError::InvalidAccessor(format!("accessor {index} is out of range"))
    })?;
    if value.get("sparse").is_some() {
        return Err(CharacterEvaluationError::SparseAccessor(index.to_string()));
    }
    crate::accessor_info(document, binary, index).map_err(map_glb_error)
}

fn attribute_index(
    attributes: &serde_json::Map<String, Value>,
    name: &str,
    primitive: usize,
) -> Result<usize, CharacterEvaluationError> {
    attributes
        .get(name)
        .and_then(Value::as_u64)
        .and_then(|index| usize::try_from(index).ok())
        .ok_or_else(|| {
            CharacterEvaluationError::InvalidAccessor(format!(
                "primitive {primitive} is missing {name}"
            ))
        })
}

fn array<'a>(
    document: &'a Value,
    key: &str,
    label: &str,
) -> Result<&'a [Value], CharacterEvaluationError> {
    crate::array_or_empty(document, key)
        .map_err(|error| CharacterEvaluationError::InvalidGlb(format!("{label}: {error}")))
}

fn scene_roots(
    document: &Value,
    nodes: &[Value],
    parents: &[Option<usize>],
) -> Result<Vec<usize>, CharacterEvaluationError> {
    let scenes = array(document, "scenes", "scenes")?;
    if scenes.is_empty() {
        if document.get("scene").is_some() {
            return Err(CharacterEvaluationError::InvalidHierarchy(
                "active scene is set but no scenes are present".into(),
            ));
        }
        return Ok(parents
            .iter()
            .enumerate()
            .filter_map(|(index, parent)| parent.is_none().then_some(index))
            .collect());
    }
    let scene_index = if document.get("scene").is_some() {
        index_field(document, "scene").ok_or_else(|| {
            CharacterEvaluationError::InvalidHierarchy("active scene index is invalid".into())
        })?
    } else {
        0
    };
    let scene = scenes.get(scene_index).ok_or_else(|| {
        CharacterEvaluationError::InvalidHierarchy("active scene index is out of range".into())
    })?;
    let roots = scene
        .get("nodes")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            CharacterEvaluationError::InvalidHierarchy("scene roots are missing".into())
        })?;
    let mut result = Vec::with_capacity(roots.len());
    let mut seen = BTreeSet::new();
    for value in roots {
        let root = value
            .as_u64()
            .and_then(|index| usize::try_from(index).ok())
            .filter(|index| *index < nodes.len())
            .ok_or_else(|| {
                CharacterEvaluationError::InvalidHierarchy("scene root index is invalid".into())
            })?;
        if parents[root].is_some() || !seen.insert(root) {
            return Err(CharacterEvaluationError::InvalidHierarchy(
                "scene roots must be unique top-level nodes".into(),
            ));
        }
        result.push(root);
    }
    Ok(result)
}

fn unique_node(nodes: &[Value], name: &str) -> Result<usize, CharacterEvaluationError> {
    if name.trim().is_empty() {
        return Err(CharacterEvaluationError::InvalidSelection(
            "node names must be non-empty".into(),
        ));
    }
    let matches = nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| node.get("name").and_then(Value::as_str) == Some(name))
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [index] => Ok(*index),
        [] => Err(CharacterEvaluationError::InvalidSelection(format!(
            "node {name} is missing"
        ))),
        _ => Err(CharacterEvaluationError::AmbiguousSelection(format!(
            "node name {name} occurs more than once"
        ))),
    }
}

fn index_field(value: &Value, field: &str) -> Option<usize> {
    value
        .get(field)
        .and_then(Value::as_u64)
        .and_then(|index| usize::try_from(index).ok())
}

fn morph_weights(value: &Value, label: &str) -> Result<bool, CharacterEvaluationError> {
    let Some(weights) = value.get("weights") else {
        return Ok(false);
    };
    let weights = weights.as_array().ok_or_else(|| {
        CharacterEvaluationError::InvalidSelection(format!("{label} weights must be an array"))
    })?;
    Ok(!weights.is_empty())
}

fn validate_child_lists(nodes: &[Value]) -> Result<(), CharacterEvaluationError> {
    for (parent, node) in nodes.iter().enumerate() {
        let Some(children) = node.get("children") else {
            continue;
        };
        let children = children.as_array().ok_or_else(|| {
            CharacterEvaluationError::InvalidHierarchy(format!(
                "node {parent} children must be an array"
            ))
        })?;
        let mut seen = BTreeSet::new();
        for child in children {
            let index = child.as_u64().ok_or_else(|| {
                CharacterEvaluationError::InvalidHierarchy(format!(
                    "node {parent} has a non-integer child index"
                ))
            })?;
            if !seen.insert(index) {
                return Err(CharacterEvaluationError::InvalidHierarchy(format!(
                    "node {parent} repeats child {index}"
                )));
            }
        }
    }
    Ok(())
}

fn is_descendant(node: usize, ancestor: usize, parents: &[Option<usize>]) -> bool {
    let mut current = Some(node);
    while let Some(index) = current {
        if index == ancestor {
            return true;
        }
        current = parents.get(index).copied().flatten();
    }
    false
}

fn validate_hierarchy_depth(parents: &[Option<usize>]) -> Result<(), CharacterEvaluationError> {
    for start in 0..parents.len() {
        let mut depth = 0;
        let mut current = Some(start);
        while let Some(node) = current {
            depth += 1;
            if depth > MAX_CPU_CHARACTER_HIERARCHY_DEPTH {
                return Err(CharacterEvaluationError::UnsupportedFeature(format!(
                    "CPU character hierarchy depth limit is {MAX_CPU_CHARACTER_HIERARCHY_DEPTH}"
                )));
            }
            current = parents[node];
        }
    }
    Ok(())
}

fn compute_globals(
    transforms: &[NodeTransform],
    parents: &[Option<usize>],
    roots: &[usize],
    active: &BTreeSet<usize>,
) -> Result<Vec<Mat4>, CharacterEvaluationError> {
    let mut globals = vec![None; transforms.len()];
    for root in roots {
        compute_global(*root, transforms, parents, active, &mut globals)?;
    }
    for node in active {
        compute_global(*node, transforms, parents, active, &mut globals)?;
    }
    if active.iter().any(|node| globals[*node].is_none()) {
        return Err(CharacterEvaluationError::InvalidHierarchy(
            "an active node is unreachable from the selected scene roots".into(),
        ));
    }
    Ok(globals
        .into_iter()
        .enumerate()
        .map(|(index, matrix)| {
            matrix.unwrap_or_else(|| {
                if active.contains(&index) {
                    unreachable!("active nodes were checked above")
                }
                IDENTITY
            })
        })
        .collect())
}

fn compute_global(
    node: usize,
    transforms: &[NodeTransform],
    parents: &[Option<usize>],
    active: &BTreeSet<usize>,
    globals: &mut [Option<Mat4>],
) -> Result<Mat4, CharacterEvaluationError> {
    if !active.contains(&node) {
        return Err(CharacterEvaluationError::InvalidHierarchy(format!(
            "node {node} is outside the active scene"
        )));
    }
    if let Some(matrix) = globals[node] {
        return Ok(matrix);
    }
    let local = transforms[node].matrix();
    if !local.iter().all(|value| value.is_finite()) {
        return Err(CharacterEvaluationError::InvalidTransform(format!(
            "node {node} local matrix is non-finite"
        )));
    }
    let global = match parents[node] {
        Some(parent) => multiply(
            compute_global(parent, transforms, parents, active, globals)?,
            local,
        ),
        None => local,
    };
    if !global.iter().all(|value| value.is_finite()) {
        return Err(CharacterEvaluationError::InvalidTransform(format!(
            "node {node} global matrix is non-finite"
        )));
    }
    globals[node] = Some(global);
    Ok(global)
}

fn trs_matrix(translation: Vec3, rotation: Quat, scale: Vec3) -> Mat4 {
    let [x, y, z, w] = rotation;
    let [sx, sy, sz] = scale;
    [
        (1.0 - 2.0 * (y * y + z * z)) * sx,
        (2.0 * (x * y + z * w)) * sx,
        (2.0 * (x * z - y * w)) * sx,
        0.0,
        (2.0 * (x * y - z * w)) * sy,
        (1.0 - 2.0 * (x * x + z * z)) * sy,
        (2.0 * (y * z + x * w)) * sy,
        0.0,
        (2.0 * (x * z + y * w)) * sz,
        (2.0 * (y * z - x * w)) * sz,
        (1.0 - 2.0 * (x * x + y * y)) * sz,
        0.0,
        translation[0],
        translation[1],
        translation[2],
        1.0,
    ]
}

fn multiply(left: Mat4, right: Mat4) -> Mat4 {
    let mut output = [0.0; 16];
    for column in 0..4 {
        for row in 0..4 {
            output[column * 4 + row] = (0..4)
                .map(|inner| left[inner * 4 + row] * right[column * 4 + inner])
                .sum();
        }
    }
    output
}

fn transform_point(matrix: Mat4, point: Vec3) -> Vec3 {
    [
        matrix[0] * point[0] + matrix[4] * point[1] + matrix[8] * point[2] + matrix[12],
        matrix[1] * point[0] + matrix[5] * point[1] + matrix[9] * point[2] + matrix[13],
        matrix[2] * point[0] + matrix[6] * point[1] + matrix[10] * point[2] + matrix[14],
    ]
}

fn inverse(matrix: Mat4) -> Option<Mat4> {
    let mut rows = [[0.0; 8]; 4];
    for row in 0..4 {
        for column in 0..4 {
            rows[row][column] = matrix[column * 4 + row];
        }
        rows[row][4 + row] = 1.0;
    }
    for column in 0..4 {
        let pivot = (column..4).max_by(|left, right| {
            rows[*left][column]
                .abs()
                .total_cmp(&rows[*right][column].abs())
        })?;
        let pivot_value = rows[pivot][column];
        if !pivot_value.is_finite() || pivot_value.abs() < 1e-12 {
            return None;
        }
        rows.swap(column, pivot);
        for value in &mut rows[column] {
            *value /= pivot_value;
        }
        let pivot_row = rows[column];
        for (row_index, row) in rows.iter_mut().enumerate() {
            if row_index == column {
                continue;
            }
            let factor = row[column];
            for entry in 0..8 {
                row[entry] -= factor * pivot_row[entry];
            }
        }
    }
    let mut output = [0.0; 16];
    for row in 0..4 {
        for column in 0..4 {
            output[column * 4 + row] = rows[row][4 + column];
        }
    }
    output
        .iter()
        .all(|value| value.is_finite())
        .then_some(output)
}

fn normalize_quat(quaternion: Quat) -> Option<Quat> {
    let norm = quaternion
        .iter()
        .map(|value| value * value)
        .sum::<f64>()
        .sqrt();
    if !norm.is_finite() || norm < 1e-12 {
        return None;
    }
    if (norm - 1.0).abs() > 1e-3 {
        return None;
    }
    Some(quaternion.map(|value| value / norm))
}

fn slerp(start: Quat, mut end: Quat, amount: f64) -> Quat {
    let mut dot = start
        .iter()
        .zip(end)
        .map(|(left, right)| left * right)
        .sum::<f64>();
    if dot < 0.0 {
        end = end.map(|value| -value);
        dot = -dot;
    }
    if dot > 0.9995 {
        return normalize_quat_unchecked(std::array::from_fn(|index| {
            start[index] * (1.0 - amount) + end[index] * amount
        }));
    }
    let angle = dot.clamp(-1.0, 1.0).acos();
    let denominator = angle.sin();
    let left = ((1.0 - amount) * angle).sin() / denominator;
    let right = (amount * angle).sin() / denominator;
    normalize_quat_unchecked(std::array::from_fn(|index| {
        start[index] * left + end[index] * right
    }))
}

fn normalize_quat_unchecked(quaternion: Quat) -> Quat {
    let norm = quaternion
        .iter()
        .map(|value| value * value)
        .sum::<f64>()
        .sqrt();
    quaternion.map(|value| value / norm)
}

fn bounds(positions: &[Vec3]) -> Bounds3 {
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for position in positions {
        for axis in 0..3 {
            min[axis] = min[axis].min(position[axis]);
            max[axis] = max[axis].max(position[axis]);
        }
    }
    Bounds3 {
        min,
        max,
        size: std::array::from_fn(|axis| max[axis] - min[axis]),
    }
}

fn map_glb_error(error: AssetContractError) -> CharacterEvaluationError {
    match error {
        AssetContractError::Unsupported(detail) if detail.contains("external buffer URI") => {
            CharacterEvaluationError::ExternalBuffer
        }
        AssetContractError::Unsupported(detail) if detail.contains("sparse accessor") => {
            CharacterEvaluationError::SparseAccessor(detail)
        }
        AssetContractError::Unsupported(detail) => {
            CharacterEvaluationError::UnsupportedFeature(detail)
        }
        other => CharacterEvaluationError::InvalidGlb(other.to_string()),
    }
}
