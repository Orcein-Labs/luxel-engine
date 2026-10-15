//! Optional view-depth diagnostics. These are separate from the promoted
//! colour receipt and never turn artistic quality into a pass/fail score.
use crate::{GraphicsContractError, GraphicsScenePacket, sha256_prefixed};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DepthCapturePayload {
    pub schema: String,
    pub packet_sha256: String,
    pub capture_id: String,
    pub width_px: u32,
    pub height_px: u32,
    pub depth_sha256: String,
    pub depth_base64: String,
}

/// Constructed only after byte, dimension, range and packet checks.
#[derive(Debug)]
pub struct VerifiedDepthCapture {
    payload: DepthCapturePayload,
    bytes: Vec<u8>,
    meters: Vec<f32>,
}
impl VerifiedDepthCapture {
    pub fn payload(&self) -> &DepthCapturePayload {
        &self.payload
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn meters(&self) -> &[f32] {
        &self.meters
    }
    pub fn verify(
        packet: &GraphicsScenePacket,
        payload: DepthCapturePayload,
    ) -> Result<Self, GraphicsContractError> {
        if payload.schema != "luxel.view-depth/v1"
            || payload.packet_sha256 != packet.packet_sha256
            || payload.capture_id != packet.body.capture.capture_id
            || payload.width_px != packet.body.capture.width_px
            || payload.height_px != packet.body.capture.height_px
        {
            return Err(GraphicsContractError::provenance(
                "depth diagnostic is detached from packet/camera/capture",
            ));
        }
        let expected = (payload.width_px as usize)
            .checked_mul(payload.height_px as usize)
            .and_then(|n| n.checked_mul(4))
            .filter(|n| *n <= crate::MAX_CAPTURE_BYTES)
            .ok_or_else(|| {
                GraphicsContractError::provenance("depth diagnostic dimensions exceed bounds")
            })?;
        // Reject oversized base64 before decoding and allocating a second copy.
        if payload.depth_base64.len() > expected.div_ceil(3) * 4 {
            return Err(GraphicsContractError::provenance(
                "depth diagnostic base64 exceeds bound",
            ));
        }
        let bytes = STANDARD
            .decode(&payload.depth_base64)
            .map_err(|_| GraphicsContractError::provenance("depth diagnostic is not base64"))?;
        if bytes.len() != expected || sha256_prefixed(&bytes) != payload.depth_sha256 {
            return Err(GraphicsContractError::provenance(
                "depth diagnostic bytes/digest mismatch",
            ));
        }
        let meters: Vec<f32> = bytes
            .chunks_exact(4)
            .map(|b| f32::from_le_bytes(b.try_into().expect("four-byte chunk")))
            .collect();
        let near = packet.body.camera.near_plane_m;
        let far = packet.body.camera.far_plane_m;
        if meters
            .iter()
            .any(|d| !d.is_finite() || *d < near * 0.999 || *d > far * 1.001)
        {
            return Err(GraphicsContractError::provenance(
                "depth diagnostic contains non-finite/out-of-camera-range samples",
            ));
        }
        Ok(Self {
            payload,
            bytes,
            meters,
        })
    }
}

/// A conservative contact measurement for visible bases of named ground props.
/// Hidden bases stay unobserved; crown meshes and suspended ruin fragments are
/// excluded. This measures geometric contact, not the appearance of AO.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GroundingSample {
    pub instance_id: String,
    pub gap_mm: i32,
    pub depth_error_mm: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GroundingEvidenceBody {
    pub schema: String,
    pub packet_sha256: String,
    pub capture_id: String,
    pub color_capture_sha256: String,
    pub depth_sha256: String,
    pub candidate_count: usize,
    pub unobserved_count: usize,
    pub contact_fraction_bp: Option<u16>,
    pub samples: Vec<GroundingSample>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GroundingEvidence {
    pub body: GroundingEvidenceBody,
    pub evidence_sha256: String,
}

pub fn measure_grounding(
    packet: &GraphicsScenePacket,
    color_bytes: &[u8],
    depth: &VerifiedDepthCapture,
) -> Result<GroundingEvidence, GraphicsContractError> {
    if depth.payload.packet_sha256 != packet.packet_sha256 || color_bytes.len() != depth.bytes.len()
    {
        return Err(GraphicsContractError::provenance(
            "grounding inputs differ from depth packet/capture",
        ));
    }
    let terrain = crate::still_water::TerrainSurface::of(&packet.body.terrain)?;
    let camera = &packet.body.camera;
    let (forward, _, _) = crate::camera_basis(camera);
    let width = camera.width_px as i32;
    let height = camera.height_px as i32;
    let mut samples = Vec::new();
    let mut candidate_count = 0;
    let meshes: std::collections::BTreeMap<_, _> = packet
        .body
        .meshes
        .iter()
        .map(|mesh| (mesh.mesh_id.as_str(), mesh))
        .collect();
    for instance in &packet.body.instances {
        let id = &instance.instance_id;
        if !(id.starts_with("kit-rock-")
            || id.starts_with("kit-tree-s")
            || id.starts_with("kit-fern-s")
            || (id.starts_with("heartwood-tree-") && id.ends_with("-trunk"))
            || id.starts_with("heartwood-understory-"))
        {
            continue;
        }
        candidate_count += 1;
        let Some(mesh) = meshes.get(instance.mesh_id.as_str()) else {
            continue;
        };
        let mut points: Vec<_> = mesh
            .positions_m
            .iter()
            .map(|position| {
                let p =
                    crate::visual_quality::transform_instance_point(&instance.transform, *position);
                (p[1] - terrain.height(p[0], p[2]), p)
            })
            .filter(|(_, p)| {
                p[0].abs() <= packet.body.terrain.width_m * 0.5
                    && p[2].abs() <= packet.body.terrain.length_m * 0.5
            })
            .collect();
        points.sort_by(|a, b| a.0.total_cmp(&b.0));
        let Some((lowest_gap, _)) = points.first() else {
            continue;
        };
        let base_band = *lowest_gap + 0.08;
        let mut observed: Option<(f32, f32)> = None;
        // Only geometry in the lowest 8 cm qualifies as the base. Depth must
        // agree to 15 cm in a 3x3 neighborhood; closer occluders never qualify.
        for (gap, point) in points.iter().take_while(|(gap, _)| *gap <= base_band) {
            let Some((x, y, _)) = crate::project_screen_point(camera, *point) else {
                continue;
            };
            let relative = std::array::from_fn(|i| point[i] - camera.position_xyz_m[i]);
            let expected_depth = crate::dot3(forward, relative);
            let x = x.floor() as i32;
            let y = y.floor() as i32;
            if !(1..width - 1).contains(&x) || !(1..height - 1).contains(&y) {
                continue;
            }
            let mut error = f32::INFINITY;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let sample = depth.meters[((y + dy) * width + x + dx) as usize];
                    error = error.min((sample - expected_depth).abs());
                }
            }
            if error <= 0.15 && observed.is_none_or(|(_, previous)| error < previous) {
                observed = Some((*gap, error));
            }
        }
        if let Some((gap, error)) = observed {
            samples.push(GroundingSample {
                instance_id: id.clone(),
                gap_mm: (gap * 1000.0).round() as i32,
                depth_error_mm: (error * 1000.0).round() as u32,
            });
        }
    }
    let touching = samples
        .iter()
        .filter(|sample| sample.gap_mm.unsigned_abs() <= 150)
        .count();
    let contact_fraction_bp =
        (!samples.is_empty()).then(|| ((touching * 10_000) / samples.len()) as u16);
    let body = GroundingEvidenceBody {
        schema: "luxel.grounding-evidence/v1".into(),
        packet_sha256: packet.packet_sha256.clone(),
        capture_id: packet.body.capture.capture_id.clone(),
        color_capture_sha256: sha256_prefixed(color_bytes),
        depth_sha256: depth.payload.depth_sha256.clone(),
        candidate_count,
        unobserved_count: candidate_count - samples.len(),
        contact_fraction_bp,
        samples,
    };
    let evidence_sha256 = sha256_prefixed(&crate::canonical_json(&body)?);
    Ok(GroundingEvidence {
        body,
        evidence_sha256,
    })
}
