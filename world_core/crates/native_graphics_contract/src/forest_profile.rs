//! Opt-in Heartwood forest-settlement content for Campaign 2.
//!
//! This is a packet content profile, not a change to the renderer or the
//! historical Campaign 2 default. It replaces the repeated N-7 tree/fern
//! instances with grounded, grouped old-growth silhouettes, understory and a
//! visible woodland path. The caller supplies the same three camera origins
//! used by the Campaign 2 scatter so this profile is view invariant.

use crate::still_water::TerrainSurface;
use crate::{
    AlphaMode, GraphicsContractError, GraphicsScenePacketBody, InstanceImportance, InstancePacket,
    InstanceVariation, MaterialIntent, MeshPacket, TextureColorSpace, Transform3d,
};
use crate::{kit, scatter};

pub const HEARTWOOD_INSTANCE_PREFIX: &str = "heartwood-";
const OLD_SCATTER_TREE_PREFIX: &str = "kit-tree-s";
const OLD_SCATTER_FERN_PREFIX: &str = "kit-fern-s";
const PROFILE_MATERIAL_PREFIX: &str = "heartwood-material-";
const PROFILE_MESH_PREFIX: &str = "heartwood-mesh-";
const CUTOUT_TEXTURE_ID: &str = "heartwood-foliage-cutout";
const CUTOUT_MATERIAL_ID: &str = "heartwood-material-foliage-cutout";
const FOREST_SEED: u64 = 0x4845_4152_5457_4F4F;
const MAX_PLACED_TREES: usize = 1_200;
const MAX_PLACED_UNDERSTORY: usize = 1_500;
const MAX_FLOOR_PATCHES: usize = 480;

fn is_scattered_instance_id(id: &str, prefix: &str) -> bool {
    let Some(rest) = id.strip_prefix(prefix) else {
        return false;
    };
    let Some((placement, mesh)) = rest.split_once('-') else {
        return false;
    };
    placement.len() == 4
        && placement.bytes().all(|byte| byte.is_ascii_digit())
        && mesh.len() == 2
        && mesh.bytes().all(|byte| byte.is_ascii_digit())
}

fn is_replaced_vegetation_instance(id: &str) -> bool {
    id.starts_with(HEARTWOOD_INSTANCE_PREFIX)
        || is_scattered_instance_id(id, OLD_SCATTER_TREE_PREFIX)
        || is_scattered_instance_id(id, OLD_SCATTER_FERN_PREFIX)
}

fn is_legacy_scatter_instance(id: &str) -> bool {
    is_scattered_instance_id(id, OLD_SCATTER_TREE_PREFIX)
        || is_scattered_instance_id(id, OLD_SCATTER_FERN_PREFIX)
}

/// Named, typed content profiles. New profiles stay opt-in at their lowering
/// seam so the absent/default packet remains byte-identical.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForestProfile {
    Heartwood,
}

/// Counts from applying a profile, useful to receipts and CPU-only tests.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ForestProfileReceipt {
    pub removed_legacy_instances: usize,
    pub added_instances: usize,
    pub tree_instances: usize,
    pub understory_instances: usize,
    pub floor_patch_count: usize,
    /// Redwood, hemlock, broadleaf.
    pub species_counts: [usize; 3],
    /// Sapling, young, mature, elder.
    pub age_counts: [usize; 4],
    pub packet_geometry_bytes: usize,
}

/// Replace the old tree and fern scatter with the Heartwood content profile.
/// Existing rocks, the ruin, standing water, terrain, backdrop and all other
/// packet content are retained. Calls are deterministic and idempotent.
pub fn apply_forest_profile(
    body: &mut GraphicsScenePacketBody,
    profile: ForestProfile,
    anchor_xyz_m: [f32; 3],
    camera_positions_xyz_m: &[[f32; 3]],
) -> Result<ForestProfileReceipt, GraphicsContractError> {
    validate_inputs(body, anchor_xyz_m, camera_positions_xyz_m)?;
    match profile {
        ForestProfile::Heartwood => apply_heartwood(body, anchor_xyz_m, camera_positions_xyz_m),
    }
}

fn validate_inputs(
    body: &GraphicsScenePacketBody,
    anchor: [f32; 3],
    cameras: &[[f32; 3]],
) -> Result<(), GraphicsContractError> {
    if anchor
        .iter()
        .any(|v| !v.is_finite() || v.abs() > crate::MAX_NATIVE_COORDINATE_M)
        || cameras.is_empty()
        || cameras.len() > 16
    {
        return Err(GraphicsContractError::malformed(
            "Heartwood needs a bounded finite ruin anchor and 1 to 16 camera origins",
        ));
    }
    if cameras
        .iter()
        .flatten()
        .any(|v| !v.is_finite() || v.abs() > crate::MAX_NATIVE_COORDINATE_M)
    {
        return Err(GraphicsContractError::malformed(
            "Heartwood camera origins must be finite",
        ));
    }
    let terrain = &body.terrain;
    if !terrain.width_m.is_finite()
        || !terrain.length_m.is_finite()
        || terrain.width_m <= 12.0
        || terrain.length_m <= 12.0
        || anchor[0].abs() > terrain.width_m * 0.5
        || anchor[2].abs() > terrain.length_m * 0.5
    {
        return Err(GraphicsContractError::malformed(
            "Heartwood anchor must lie inside a finite, positive terrain extent",
        ));
    }
    let camera_mean = cameras.iter().fold([0.0, 0.0], |sum, camera| {
        [
            sum[0] + camera[0] / cameras.len() as f32,
            sum[1] + camera[2] / cameras.len() as f32,
        ]
    });
    if (camera_mean[0] - anchor[0]).hypot(camera_mean[1] - anchor[2]) < 1.0 {
        return Err(GraphicsContractError::malformed(
            "Heartwood camera origins must define a path heading away from the ruin",
        ));
    }
    Ok(())
}

fn apply_heartwood(
    body: &mut GraphicsScenePacketBody,
    anchor: [f32; 3],
    cameras: &[[f32; 3]],
) -> Result<ForestProfileReceipt, GraphicsContractError> {
    let surface = TerrainSurface::of(&body.terrain)?;
    let previous_instance_count = body.instances.len();
    let old_tree_material_ids: std::collections::BTreeSet<&str> = body
        .instances
        .iter()
        .filter(|instance| is_scattered_instance_id(&instance.instance_id, OLD_SCATTER_TREE_PREFIX))
        .map(|instance| instance.material_id.as_str())
        .collect();
    let reusable_tree_cutout = body
        .materials
        .iter()
        .find(|material| {
            old_tree_material_ids.contains(material.material_id.as_str())
                && material.alpha_mode == AlphaMode::Mask
                && !material.texture_ids.is_empty()
                && material.texture_ids.iter().all(|id| {
                    body.textures
                        .iter()
                        .any(|texture| texture.texture_id == *id)
                })
        })
        .cloned();
    let reusable_profile_cutout = body
        .materials
        .iter()
        .find(|material| {
            material.material_id == CUTOUT_MATERIAL_ID
                && material.alpha_mode == AlphaMode::Mask
                && !material.texture_ids.is_empty()
                && material.texture_ids.iter().all(|id| {
                    body.textures
                        .iter()
                        .any(|texture| texture.texture_id == *id)
                })
        })
        .cloned();
    let reusable_cutout = reusable_tree_cutout.or(reusable_profile_cutout);
    let removed_resources: Vec<(String, String)> = body
        .instances
        .iter()
        .filter(|instance| is_legacy_scatter_instance(&instance.instance_id))
        .map(|instance| (instance.mesh_id.clone(), instance.material_id.clone()))
        .collect();
    body.instances
        .retain(|instance| !is_replaced_vegetation_instance(&instance.instance_id));
    let removed_legacy_instances = previous_instance_count - body.instances.len();
    body.meshes
        .retain(|mesh| !mesh.mesh_id.starts_with(PROFILE_MESH_PREFIX));
    body.materials
        .retain(|material| !material.material_id.starts_with(PROFILE_MATERIAL_PREFIX));
    let needs_profile_cutout_texture = reusable_cutout.as_ref().is_some_and(|material| {
        material
            .texture_ids
            .iter()
            .any(|id| id == CUTOUT_TEXTURE_ID)
    });
    body.textures
        .retain(|texture| texture.texture_id != CUTOUT_TEXTURE_ID || needs_profile_cutout_texture);
    if let Some(mut source) = reusable_cutout {
        source.material_id = CUTOUT_MATERIAL_ID.into();
        source.double_sided = Some(true);
        body.materials.push(source);
    } else {
        body.textures.push(cutout_leaf_texture());
        body.materials.push(cutout_leaf_material());
    }
    prune_replaced_vegetation_resources(body, &removed_resources);

    add_materials(body);
    add_meshes(body)?;

    let camera_xz: Vec<[f32; 2]> = cameras.iter().map(|c| [c[0], c[2]]).collect();
    let path = path_geometry(
        &surface,
        body.terrain.width_m,
        body.terrain.length_m,
        anchor,
        &camera_xz,
    );
    body.meshes.push(path.mesh.clone());
    body.instances.push(instance(
        "heartwood-path-ribbon",
        &path.mesh.mesh_id,
        &path.mesh.material_id,
        [0.0, 0.0, 0.0],
        [1.0, 1.0, 1.0],
        [0.0, 0.0, 0.0, 1.0],
        None,
    ));

    let exclusions = exclusions(body, anchor, &camera_xz, &path.corridors);
    let half_x = body.terrain.width_m * 0.5 - 6.0;
    let half_z = body.terrain.length_m * 0.5 - 6.0;
    if half_x <= 0.0 || half_z <= 0.0 {
        return Err(GraphicsContractError::unsupported(
            "Heartwood needs at least 12 m of terrain in each horizontal dimension",
        ));
    }
    let lo = [-half_x, -half_z];
    let hi = [half_x, half_z];
    let hero = [anchor[0], anchor[2]];
    let forest_density = |x: f32, z: f32| {
        let distance = ((x - hero[0]).powi(2) + (z - hero[1]).powi(2)).sqrt();
        let grove = 0.22
            + 0.78 * scatter::smoothstep(0.34, 0.62, scatter::value_noise(x, z, 35.0, 0x4845_4152));
        let clearing = scatter::smoothstep(9.0, 24.0, distance);
        clearing * grove
    };
    let candidates = scatter::poisson_disc(lo, hi, 5.0, FOREST_SEED);
    let mut receipt = ForestProfileReceipt {
        removed_legacy_instances,
        ..ForestProfileReceipt::default()
    };
    for (index, [x, z]) in candidates.into_iter().enumerate() {
        if receipt.tree_instances >= MAX_PLACED_TREES
            || forest_density(x, z) < scatter::value_noise(x, z, 7.0, 0x5452_4545)
            || scatter::slope(&surface, x, z) > 0.62
            || exclusions
                .iter()
                .any(|exclusion| exclusion.contains([x, z]))
        {
            continue;
        }
        let species_value = scatter::value_noise(x, z, 29.0, 0x5350_4543);
        let species = if species_value < 0.48 {
            Species::Redwood
        } else if species_value < 0.72 {
            Species::Hemlock
        } else {
            Species::Broadleaf
        };
        let age_value = scatter::value_noise(x, z, 23.0, 0x4147_4553);
        let age = if age_value < 0.22 {
            Age::Sapling
        } else if age_value < 0.48 {
            Age::Young
        } else if age_value < 0.78 {
            Age::Mature
        } else {
            Age::Elder
        };
        let local_variation = scatter::value_noise(x, z, 2.7, 0x5349_5A45);
        let scale = age.scale() * (0.88 + local_variation * 0.24);
        let yaw = scatter::value_noise(x, z, 1.0, 0x5941_5721) * std::f32::consts::TAU;
        let ground = surface.height(x, z);
        let trunk_id = species.trunk_mesh();
        let crown_id = species.crown_mesh();
        let trunk_material = species.trunk_material();
        let foliage_material = species.foliage_material();
        let spin = yaw_quaternion(yaw);
        let tint = age.tint();
        let variation = Some(InstanceVariation { tint_rgb: tint });
        body.instances.push(instance(
            &format!("heartwood-tree-{index:04}-trunk"),
            trunk_id,
            trunk_material,
            [x, ground, z],
            [scale; 3],
            spin,
            variation,
        ));
        let crown_lift = (species.trunk_height() - species.crown_height()) * scale;
        body.instances.push(instance(
            &format!("heartwood-tree-{index:04}-crown"),
            crown_id,
            foliage_material,
            [x, ground + crown_lift, z],
            [scale; 3],
            spin,
            variation,
        ));
        receipt.tree_instances += 1;
        receipt.species_counts[species.index()] += 1;
        receipt.age_counts[age.index()] += 1;
    }

    let understory = scatter::poisson_disc(lo, hi, 2.3, FOREST_SEED ^ 0x554E_4445);
    for (index, [x, z]) in understory.into_iter().enumerate() {
        if receipt.understory_instances >= MAX_PLACED_UNDERSTORY
            || scatter::value_noise(x, z, 7.5, 0x554E_4445) < 0.57
            || scatter::value_noise(x, z, 28.0, 0x4752_4F57) < 0.24
            || scatter::slope(&surface, x, z) > 0.78
            || exclusions
                .iter()
                .any(|exclusion| exclusion.contains([x, z]))
        {
            continue;
        }
        let scale = 0.65 + scatter::value_noise(x, z, 1.7, 0x4255_5348) * 0.9;
        let yaw = scatter::value_noise(x, z, 1.0, 0x554E_4459) * std::f32::consts::TAU;
        body.instances.push(instance(
            &format!("heartwood-understory-{index:04}"),
            "heartwood-mesh-understory",
            CUTOUT_MATERIAL_ID,
            [x, surface.height(x, z), z],
            [scale; 3],
            yaw_quaternion(yaw),
            Some(InstanceVariation {
                tint_rgb: [0.82 + scale * 0.1, 0.93, 0.75 + scale * 0.12],
            }),
        ));
        receipt.understory_instances += 1;
    }

    let floor_mesh = floor_patches(&surface, lo, hi, &exclusions, FOREST_SEED ^ 0x4C49_5454);
    receipt.floor_patch_count = floor_mesh.positions_m.len() / 9;
    if !floor_mesh.indices.is_empty() {
        body.instances.push(instance(
            "heartwood-floor-litter",
            &floor_mesh.mesh_id,
            &floor_mesh.material_id,
            [0.0, 0.0, 0.0],
            [1.0; 3],
            [0.0, 0.0, 0.0, 1.0],
            None,
        ));
        body.meshes.push(floor_mesh);
    }
    receipt.added_instances = body
        .instances
        .iter()
        .filter(|instance| instance.instance_id.starts_with(HEARTWOOD_INSTANCE_PREFIX))
        .count();
    receipt.packet_geometry_bytes = body
        .meshes
        .iter()
        .filter(|mesh| mesh.mesh_id.starts_with(PROFILE_MESH_PREFIX))
        .map(mesh_geometry_bytes)
        .sum();
    Ok(receipt)
}

/// Drop only mesh/material/texture resources made dead by removing the old
/// tree and fern instances. The kit's rock and ruin resources stay referenced
/// and are preserved.
fn prune_replaced_vegetation_resources(
    body: &mut GraphicsScenePacketBody,
    candidates: &[(String, String)],
) {
    let candidate_meshes: std::collections::BTreeSet<&str> =
        candidates.iter().map(|(mesh, _)| mesh.as_str()).collect();
    let candidate_materials: std::collections::BTreeSet<&str> = candidates
        .iter()
        .map(|(_, material)| material.as_str())
        .collect();
    let used_meshes: std::collections::BTreeSet<&str> =
        body.instances.iter().map(|i| i.mesh_id.as_str()).collect();
    body.meshes.retain(|mesh| {
        !candidate_meshes.contains(mesh.mesh_id.as_str())
            || used_meshes.contains(mesh.mesh_id.as_str())
    });
    let used_materials: std::collections::BTreeSet<&str> = body
        .instances
        .iter()
        .map(|i| i.material_id.as_str())
        .chain(body.meshes.iter().map(|mesh| mesh.material_id.as_str()))
        .chain(std::iter::once(body.terrain.material_id.as_str()))
        .collect();
    let removed_texture_ids: std::collections::BTreeSet<String> = body
        .materials
        .iter()
        .filter(|material| {
            candidate_materials.contains(material.material_id.as_str())
                && !used_materials.contains(material.material_id.as_str())
        })
        .flat_map(material_texture_ids)
        .collect();
    body.materials.retain(|material| {
        !candidate_materials.contains(material.material_id.as_str())
            || used_materials.contains(material.material_id.as_str())
    });
    let used_textures: std::collections::BTreeSet<String> = body
        .materials
        .iter()
        .flat_map(material_texture_ids)
        .collect();
    body.textures.retain(|texture| {
        !removed_texture_ids.contains(&texture.texture_id)
            || used_textures.contains(&texture.texture_id)
    });
}

fn material_texture_ids(material: &MaterialIntent) -> Vec<String> {
    material
        .texture_ids
        .iter()
        .cloned()
        .chain(material.normal_texture_id.iter().cloned())
        .chain(material.roughness_texture_id.iter().cloned())
        .chain(material.occlusion_texture_id.iter().cloned())
        .chain(material.emissive_texture_id.iter().cloned())
        .collect()
}

#[derive(Clone, Copy)]
enum Species {
    Redwood,
    Hemlock,
    Broadleaf,
}

impl Species {
    fn index(self) -> usize {
        match self {
            Self::Redwood => 0,
            Self::Hemlock => 1,
            Self::Broadleaf => 2,
        }
    }
    fn trunk_mesh(self) -> &'static str {
        match self {
            Self::Redwood => "heartwood-mesh-trunk-redwood",
            Self::Hemlock => "heartwood-mesh-trunk-hemlock",
            Self::Broadleaf => "heartwood-mesh-trunk-broadleaf",
        }
    }
    fn crown_mesh(self) -> &'static str {
        match self {
            Self::Redwood => "heartwood-mesh-crown-redwood",
            Self::Hemlock => "heartwood-mesh-crown-hemlock",
            Self::Broadleaf => "heartwood-mesh-crown-broadleaf",
        }
    }
    fn trunk_material(self) -> &'static str {
        match self {
            Self::Redwood => "heartwood-material-redwood-bark",
            Self::Hemlock => "heartwood-material-hemlock-bark",
            Self::Broadleaf => "heartwood-material-broadleaf-bark",
        }
    }
    fn foliage_material(self) -> &'static str {
        let _ = self;
        CUTOUT_MATERIAL_ID
    }
    fn trunk_height(self) -> f32 {
        match self {
            Self::Redwood => 27.0,
            Self::Hemlock => 16.0,
            Self::Broadleaf => 12.0,
        }
    }
    fn crown_height(self) -> f32 {
        match self {
            Self::Redwood => 14.0,
            Self::Hemlock => 10.0,
            Self::Broadleaf => 8.0,
        }
    }
    fn crown_radius(self) -> f32 {
        match self {
            Self::Redwood => 3.8,
            Self::Hemlock => 2.2,
            Self::Broadleaf => 5.2,
        }
    }
}

#[derive(Clone, Copy)]
enum Age {
    Sapling,
    Young,
    Mature,
    Elder,
}

impl Age {
    fn index(self) -> usize {
        match self {
            Self::Sapling => 0,
            Self::Young => 1,
            Self::Mature => 2,
            Self::Elder => 3,
        }
    }
    fn scale(self) -> f32 {
        match self {
            Self::Sapling => 0.34,
            Self::Young => 0.61,
            Self::Mature => 1.0,
            Self::Elder => 1.58,
        }
    }
    fn tint(self) -> [f32; 3] {
        match self {
            Self::Sapling => [0.88, 1.12, 0.84],
            Self::Young => [0.93, 1.05, 0.88],
            Self::Mature => [1.0, 1.0, 1.0],
            Self::Elder => [0.91, 0.98, 0.85],
        }
    }
}

fn add_materials(body: &mut GraphicsScenePacketBody) {
    for (name, color, roughness) in [
        ("redwood-bark", [0.23, 0.095, 0.047, 1.0], 0.88),
        ("hemlock-bark", [0.28, 0.20, 0.13, 1.0], 0.86),
        ("broadleaf-bark", [0.31, 0.25, 0.17, 1.0], 0.9),
        ("litter", [0.21, 0.16, 0.09, 1.0], 0.97),
        ("path", [0.24, 0.19, 0.125, 1.0], 0.94),
    ] {
        body.materials.push(MaterialIntent {
            material_id: format!("{PROFILE_MATERIAL_PREFIX}{name}"),
            base_color_rgba: color,
            metallic: 0.0,
            roughness,
            clearcoat: 0.0,
            clearcoat_roughness: 0.045,
            alpha_mode: AlphaMode::Opaque,
            alpha_cutoff: None,
            double_sided: Some(true),
            metallic_from_texture: None,
            texture_ids: Vec::new(),
            normal_texture_id: None,
            roughness_texture_id: None,
            occlusion_texture_id: None,
            emissive_texture_id: None,
            normal_scale: 1.0,
            occlusion_strength: 1.0,
            emissive_factor_rgb: [0.0; 3],
        });
    }
}

fn cutout_leaf_material() -> MaterialIntent {
    MaterialIntent {
        material_id: CUTOUT_MATERIAL_ID.into(),
        base_color_rgba: [0.94, 1.0, 0.9, 1.0],
        metallic: 0.0,
        roughness: 0.84,
        clearcoat: 0.02,
        clearcoat_roughness: 0.3,
        alpha_mode: AlphaMode::Mask,
        alpha_cutoff: Some(0.5),
        double_sided: Some(true),
        metallic_from_texture: None,
        texture_ids: vec![CUTOUT_TEXTURE_ID.into()],
        normal_texture_id: None,
        roughness_texture_id: None,
        occlusion_texture_id: None,
        emissive_texture_id: None,
        normal_scale: 1.0,
        occlusion_strength: 1.0,
        emissive_factor_rgb: [0.0; 3],
    }
}

/// Small deterministic atlas of opaque leaflets separated by transparent
/// gaps. It is the fallback when the pinned tree asset supplies no cutout
/// material; actual kit2's authored mask and texture are preferred.
fn cutout_leaf_texture() -> crate::TextureReference {
    const SIZE: usize = 64;
    let mut rgba = vec![0u8; SIZE * SIZE * 4];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let mut color = None;
            for leaf in 0..13 {
                let cx = 5.0 + (leaf * 23 % 53) as f32;
                let cy = 5.0 + (leaf * 37 % 53) as f32;
                let angle = leaf as f32 * 0.713;
                let (sin, cos) = angle.sin_cos();
                let px = x as f32 - cx;
                let py = y as f32 - cy;
                let u = px * cos + py * sin;
                let v = -px * sin + py * cos;
                let half_length = 3.4 + (leaf % 3) as f32 * 0.7;
                let width = 1.2 + (leaf % 2) as f32 * 0.25;
                let shape = (u / half_length).powi(2)
                    + (v / width).powi(2) * (1.0 + 0.28 * (u / half_length).abs());
                if shape <= 1.0 {
                    let shade = (leaf * 11 % 31) as u8;
                    color = Some([48 + shade, 92 + shade, 27 + shade / 2, 255]);
                    break;
                }
            }
            if let Some(pixel) = color {
                let index = (y * SIZE + x) * 4;
                rgba[index..index + 4].copy_from_slice(&pixel);
            }
        }
    }
    crate::procedural_texture(
        CUTOUT_TEXTURE_ID,
        "procedural-luxel-heartwood-leaf-cutout-v1",
        TextureColorSpace::Srgb,
        SIZE as u32,
        SIZE as u32,
        rgba,
    )
}

fn add_meshes(body: &mut GraphicsScenePacketBody) -> Result<(), GraphicsContractError> {
    let mut meshes = vec![
        trunk_mesh(
            "redwood",
            "heartwood-material-redwood-bark",
            27.0,
            0.92,
            0.18,
        ),
        trunk_mesh(
            "hemlock",
            "heartwood-material-hemlock-bark",
            16.0,
            0.48,
            0.10,
        ),
        trunk_mesh(
            "broadleaf",
            "heartwood-material-broadleaf-bark",
            12.0,
            0.72,
            0.13,
        ),
        crown_mesh(Species::Redwood),
        crown_mesh(Species::Hemlock),
        crown_mesh(Species::Broadleaf),
        understory_mesh(),
    ];
    let mut ids = std::collections::BTreeSet::new();
    for mesh in &meshes {
        if !ids.insert(mesh.mesh_id.as_str()) {
            return Err(GraphicsContractError::malformed(
                "Heartwood procedural mesh ids are not unique",
            ));
        }
    }
    body.meshes.append(&mut meshes);
    Ok(())
}

fn trunk_mesh(species: &str, material: &str, height: f32, radius: f32, taper: f32) -> MeshPacket {
    let mut builder = MeshBuilder::default();
    let segments = 12;
    let rings = 9;
    for ring in 0..rings {
        let v = ring as f32 / (rings - 1) as f32;
        let y = height * v;
        let base = radius * (1.0 - taper * v).max(0.22);
        let buttress = if v < 0.13 {
            (1.0 - v / 0.13) * 1.9
        } else {
            0.0
        };
        for segment in 0..segments {
            let angle = std::f32::consts::TAU * segment as f32 / segments as f32;
            let rib = 1.0 + 0.055 * (angle * 5.0).sin() + buttress * (angle * 4.0).cos().max(0.0);
            let r = base * rib;
            builder.vertex(
                [r * angle.cos(), y, r * angle.sin()],
                [angle.cos(), 0.12, angle.sin()],
                [segment as f32 / segments as f32, v],
            );
        }
    }
    for ring in 0..rings - 1 {
        for segment in 0..segments {
            let a = (ring * segments + segment) as u32;
            let b = (ring * segments + (segment + 1) % segments) as u32;
            let c = ((ring + 1) * segments + (segment + 1) % segments) as u32;
            let d = ((ring + 1) * segments + segment) as u32;
            builder.triangle(a, c, b);
            builder.triangle(a, d, c);
        }
    }
    // Eight flared roots merge into the buttress base, making the old trees
    // visibly belong to the ground instead of ending as smooth cylinders.
    for root in 0..8 {
        let angle = std::f32::consts::TAU * root as f32 / 8.0;
        let (dx, dz) = (angle.cos(), angle.sin());
        let center = builder.positions.len() as u32;
        builder.vertex(
            [radius * 0.36 * dx, 0.03, radius * 0.36 * dz],
            [dx, 0.18, dz],
            [0.5, 0.0],
        );
        builder.vertex(
            [radius * 1.65 * dx, 0.015, radius * 1.65 * dz],
            [dx, 0.15, dz],
            [1.0, 0.0],
        );
        builder.vertex(
            [radius * 0.55 * dx, height * 0.16, radius * 0.55 * dz],
            [dx, 0.35, dz],
            [0.5, 0.16],
        );
        builder.triangle(center, center + 1, center + 2);
    }
    builder.finish(format!("{PROFILE_MESH_PREFIX}trunk-{species}"), material)
}

fn crown_mesh(species: Species) -> MeshPacket {
    let mut builder = MeshBuilder::default();
    let levels: &[(f32, f32)] = match species {
        Species::Redwood => &[
            (0.02, 0.14),
            (0.18, 0.62),
            (0.38, 1.0),
            (0.60, 0.84),
            (0.92, 0.42),
            (1.0, 0.015),
        ],
        Species::Hemlock => &[
            (0.015, 0.08),
            (0.18, 1.0),
            (0.38, 0.82),
            (0.60, 0.60),
            (0.92, 0.29),
            (1.0, 0.01),
        ],
        Species::Broadleaf => &[
            (0.0, 0.16),
            (0.18, 0.66),
            (0.40, 1.0),
            (0.62, 0.88),
            (0.92, 0.30),
            (1.0, 0.03),
        ],
    };
    // Branch cards use metres directly. Crossed planes retain foliage volume
    // from changing camera angles while the alpha texture keeps leaf gaps open.
    let branch_count = if matches!(species, Species::Broadleaf) {
        22
    } else {
        16
    };
    for branch in 0..branch_count {
        let angle = std::f32::consts::TAU * branch as f32 / branch_count as f32;
        let noise = (angle * 3.0).sin() * 0.065 + (angle * 7.0).cos() * 0.035;
        let radial = [angle.cos(), angle.sin()];
        let tangent = [-radial[1], radial[0]];
        for (level, (height_fraction, radius_fraction)) in levels.iter().copied().enumerate() {
            if level + 1 == levels.len() {
                continue;
            }
            let wave = ((branch * 13 + level * 7) as f32 * 1.71).sin();
            let y0 = (height_fraction - 0.065 + wave * 0.018).max(0.02) * species.crown_height();
            let y1 = (height_fraction + 0.085 + wave * 0.018).min(0.98) * species.crown_height();
            let outer = species.crown_radius() * (radius_fraction + noise).max(0.08);
            let inner = outer
                * if matches!(species, Species::Redwood) {
                    0.10
                } else {
                    0.08
                };
            let start = [radial[0] * inner, y0, radial[1] * inner];
            let end = [radial[0] * outer, y1, radial[1] * outer];
            let half_width = ((outer - inner) * 0.105 + 0.16).min(0.72);
            append_branch_card(&mut builder, start, end, tangent, half_width, wave * 0.12);
            // The second plane is crossed through the first branch for broad
            // angular coverage; both carry the same alpha-cutout leaf atlas.
            append_branch_card(
                &mut builder,
                start,
                end,
                radial,
                half_width * 0.78,
                -wave * 0.12,
            );
        }
    }
    builder.finish(species.crown_mesh().to_owned(), species.foliage_material())
}

fn understory_mesh() -> MeshPacket {
    let mut builder = MeshBuilder::default();
    // A mixed fern/grass clump built from alpha-cutout leaf cards, not an
    // opaque shrub ball. Blade length, width and bend vary around the clump.
    for blade in 0..13 {
        let angle = std::f32::consts::TAU * blade as f32 / 13.0;
        let radial = [angle.cos(), angle.sin()];
        let tangent = [-radial[1], radial[0]];
        let height = 0.55 + (blade as f32 * 1.31).sin().abs() * 0.65;
        let reach = 0.30 + (blade as f32 * 0.77).cos().abs() * 0.18;
        let bend = (blade as f32 * 2.1).sin() * 0.11;
        append_branch_card(
            &mut builder,
            [radial[0] * 0.035, 0.025, radial[1] * 0.035],
            [
                radial[0] * reach + tangent[0] * bend,
                height,
                radial[1] * reach + tangent[1] * bend,
            ],
            tangent,
            0.075 + (blade % 3) as f32 * 0.025,
            bend,
        );
        append_branch_card(
            &mut builder,
            [radial[0] * 0.025, 0.02, radial[1] * 0.025],
            [
                radial[0] * reach * 0.78,
                height * 0.72,
                radial[1] * reach * 0.78,
            ],
            radial,
            0.09,
            -bend,
        );
    }
    builder.finish(
        format!("{PROFILE_MESH_PREFIX}understory"),
        CUTOUT_MATERIAL_ID,
    )
}

fn append_branch_card(
    builder: &mut MeshBuilder,
    start: [f32; 3],
    end: [f32; 3],
    width_axis: [f32; 2],
    half_width: f32,
    skew: f32,
) {
    let delta = [end[0] - start[0], end[1] - start[1], end[2] - start[2]];
    let normal = normalize3([
        -width_axis[1] * delta[1],
        width_axis[1] * delta[0] - width_axis[0] * delta[2],
        width_axis[0] * delta[1],
    ]);
    let start_half = half_width * 0.58;
    let points = [
        [
            start[0] - width_axis[0] * start_half,
            start[1],
            start[2] - width_axis[1] * start_half,
        ],
        [
            start[0] + width_axis[0] * start_half,
            start[1],
            start[2] + width_axis[1] * start_half,
        ],
        [
            end[0] + width_axis[0] * (half_width + skew),
            end[1],
            end[2] + width_axis[1] * (half_width + skew),
        ],
        [
            end[0] - width_axis[0] * (half_width - skew),
            end[1],
            end[2] - width_axis[1] * (half_width - skew),
        ],
    ];
    let base = builder.positions.len() as u32;
    for (point, uv) in points
        .into_iter()
        .zip([[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]])
    {
        builder.vertex(point, normal, uv);
    }
    builder.triangle(base, base + 1, base + 2);
    builder.triangle(base, base + 2, base + 3);
}

#[derive(Clone)]
struct PathGeometry {
    mesh: MeshPacket,
    corridors: Vec<scatter::Exclusion>,
}

fn path_geometry(
    surface: &TerrainSurface,
    terrain_width_m: f32,
    terrain_length_m: f32,
    anchor: [f32; 3],
    cameras: &[[f32; 2]],
) -> PathGeometry {
    let [target_x, target_z] = cameras
        .iter()
        .fold([0.0, 0.0], |sum, c| [sum[0] + c[0], sum[1] + c[1]])
        .map(|sum| sum / cameras.len() as f32);
    let [dx, dz] = normalize2([target_x - anchor[0], target_z - anchor[2]]);
    let side = [-dz, dx];
    let length = (terrain_width_m.min(terrain_length_m) * 0.29).clamp(26.0, 52.0);
    let segments = 24;
    let mut builder = MeshBuilder::default();
    for i in 0..=segments {
        let t = i as f32 / segments as f32;
        let bend = (t * std::f32::consts::PI).sin() * 2.2 + (t * 3.4).sin() * 0.65;
        let center = [
            anchor[0] + dx * (7.2 + length * t) + side[0] * bend,
            anchor[2] + dz * (7.2 + length * t) + side[1] * bend,
        ];
        let width = 2.3 + 0.5 * (t * std::f32::consts::TAU).sin().abs();
        for offset in [-width, width] {
            let x = center[0] + side[0] * offset;
            let z = center[1] + side[1] * offset;
            let y = surface.height(x, z) + 0.025;
            builder.vertex(
                [x, y, z],
                surface_normal(surface, x, z),
                [t, offset / (2.0 * width) + 0.5],
            );
        }
    }
    for i in 0..segments {
        let a = (i * 2) as u32;
        let b = a + 1;
        let c = a + 3;
        let d = a + 2;
        builder.triangle(a, b, c);
        builder.triangle(a, c, d);
    }
    let centers: Vec<[f32; 2]> = (0..=segments)
        .map(|i| {
            let t = i as f32 / segments as f32;
            let bend = (t * std::f32::consts::PI).sin() * 2.2 + (t * 3.4).sin() * 0.65;
            [
                anchor[0] + dx * (7.2 + length * t) + side[0] * bend,
                anchor[2] + dz * (7.2 + length * t) + side[1] * bend,
            ]
        })
        .collect();
    let corridors = centers
        .windows(2)
        .map(|window| scatter::Exclusion::Corridor {
            from: window[0],
            to: window[1],
            half_width_from: 4.0,
            half_width_to: 4.0,
        })
        .collect();
    PathGeometry {
        mesh: builder.finish(
            format!("{PROFILE_MESH_PREFIX}path-ribbon"),
            "heartwood-material-path",
        ),
        corridors,
    }
}

fn exclusions(
    body: &GraphicsScenePacketBody,
    anchor: [f32; 3],
    cameras: &[[f32; 2]],
    path: &[scatter::Exclusion],
) -> Vec<scatter::Exclusion> {
    let mut exclusions = Vec::new();
    for camera in cameras {
        exclusions.push(scatter::Exclusion::Corridor {
            from: *camera,
            to: [anchor[0], anchor[2]],
            half_width_from: 3.0,
            half_width_to: 8.5,
        });
        exclusions.push(scatter::Exclusion::Disc {
            center: *camera,
            radius: 8.0,
        });
    }
    exclusions.extend(
        kit::ruin_footprints(anchor)
            .into_iter()
            .map(|(center, radius)| scatter::Exclusion::Disc {
                center,
                radius: radius + 1.8,
            }),
    );
    if let Some(wet) = body.terrain.wet_zone {
        exclusions.push(scatter::Exclusion::Ellipse {
            center: wet.center_xz_m,
            radii: [wet.radii_xz_m[0] * 1.55, wet.radii_xz_m[1] * 1.55],
        });
    }
    exclusions.extend_from_slice(path);
    exclusions
}

fn floor_patches(
    surface: &TerrainSurface,
    lo: [f32; 2],
    hi: [f32; 2],
    exclusions: &[scatter::Exclusion],
    seed: u64,
) -> MeshPacket {
    let mut builder = MeshBuilder::default();
    let candidates = scatter::poisson_disc(lo, hi, 1.45, seed);
    let mut count = 0usize;
    for [x, z] in candidates {
        if count >= MAX_FLOOR_PATCHES
            || scatter::value_noise(x, z, 4.3, 0x4C49_5454) < 0.52
            || exclusions
                .iter()
                .any(|exclusion| exclusion.contains([x, z]))
        {
            continue;
        }
        let radius = 0.25 + scatter::value_noise(x, z, 1.2, 0x5041_5443) * 0.55;
        let angle = scatter::value_noise(x, z, 1.0, 0x4C45_4156) * std::f32::consts::TAU;
        let start = builder.positions.len() as u32;
        let y = surface.height(x, z) + 0.012;
        builder.vertex([x, y, z], surface_normal(surface, x, z), [0.5, 0.5]);
        for i in 0..8 {
            let theta = angle + std::f32::consts::TAU * i as f32 / 8.0;
            let variation =
                0.75 + scatter::value_noise(x + theta, z - theta, 0.5, 0x5041_5443) * 0.5;
            let px = x + theta.cos() * radius * variation;
            let pz = z + theta.sin() * radius * variation;
            builder.vertex(
                [px, surface.height(px, pz) + 0.014, pz],
                surface_normal(surface, px, pz),
                [0.5 + theta.cos(), 0.5 + theta.sin()],
            );
        }
        for i in 0..8 {
            builder.triangle(start, start + 1 + (i + 1) % 8, start + 1 + i);
        }
        count += 1;
    }
    builder.finish(
        format!("{PROFILE_MESH_PREFIX}litter-patches"),
        "heartwood-material-litter",
    )
}

fn surface_normal(surface: &TerrainSurface, x: f32, z: f32) -> [f32; 3] {
    let epsilon = 0.2;
    let dx = (surface.height(x + epsilon, z) - surface.height(x - epsilon, z)) / (2.0 * epsilon);
    let dz = (surface.height(x, z + epsilon) - surface.height(x, z - epsilon)) / (2.0 * epsilon);
    normalize3([-dx, 1.0, -dz])
}

fn instance(
    id: &str,
    mesh_id: &str,
    material_id: &str,
    translation: [f32; 3],
    scale: [f32; 3],
    rotation: [f32; 4],
    variation: Option<InstanceVariation>,
) -> InstancePacket {
    InstancePacket {
        instance_id: id.to_owned(),
        mesh_id: mesh_id.to_owned(),
        material_id: material_id.to_owned(),
        importance: InstanceImportance::Background,
        transform: Transform3d {
            translation_xyz_m: translation,
            rotation_xyzw: rotation,
            scale_xyz: scale,
        },
        variation,
    }
}

fn yaw_quaternion(yaw: f32) -> [f32; 4] {
    [0.0, (yaw * 0.5).sin(), 0.0, (yaw * 0.5).cos()]
}

fn normalize2(v: [f32; 2]) -> [f32; 2] {
    let length = (v[0] * v[0] + v[1] * v[1]).sqrt().max(1.0e-6);
    [v[0] / length, v[1] / length]
}

fn normalize3(v: [f32; 3]) -> [f32; 3] {
    let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1.0e-6);
    [v[0] / length, v[1] / length, v[2] / length]
}

#[derive(Default)]
struct MeshBuilder {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uv0: Vec<[f32; 2]>,
    indices: Vec<u32>,
}

impl MeshBuilder {
    fn vertex(&mut self, position: [f32; 3], normal: [f32; 3], uv: [f32; 2]) {
        self.positions.push(position);
        self.normals.push(normalize3(normal));
        self.uv0.push(uv);
    }
    fn triangle(&mut self, a: u32, b: u32, c: u32) {
        self.indices.extend([a, b, c]);
    }
    fn finish(self, mesh_id: String, material_id: &str) -> MeshPacket {
        MeshPacket {
            mesh_id,
            positions_m: self.positions,
            normals: self.normals,
            uv0: self.uv0,
            indices: self.indices,
            material_id: material_id.to_owned(),
            tangents: Vec::new(),
        }
    }
}

fn mesh_geometry_bytes(mesh: &MeshPacket) -> usize {
    mesh.positions_m.len() * std::mem::size_of::<[f32; 3]>()
        + mesh.normals.len() * std::mem::size_of::<[f32; 3]>()
        + mesh.uv0.len() * std::mem::size_of::<[f32; 2]>()
        + mesh.indices.len() * std::mem::size_of::<u32>()
}
