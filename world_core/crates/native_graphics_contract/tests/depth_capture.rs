use std::path::PathBuf;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use luxel_native_graphics_contract::depth_capture::{DepthCapturePayload, VerifiedDepthCapture};
use luxel_native_graphics_contract::{GraphicsScenePacket, sha256_prefixed};
use luxel_reference_runtime::WorldArtifact;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn fixtures() -> (&'static GraphicsScenePacket, &'static WorldArtifact) {
    static FIXTURE: std::sync::OnceLock<(GraphicsScenePacket, WorldArtifact)> =
        std::sync::OnceLock::new();
    let (packet, world) = FIXTURE.get_or_init(|| {
        let root = repo_root();
        let layout_path =
            root.join("world_core/crates/reference_runtime/examples/riverwatch.layout.json");
        let layout: luxel_reference_runtime::AuthoredLayout =
            serde_json::from_slice(&std::fs::read(&layout_path).expect("Riverwatch layout reads"))
                .expect("Riverwatch layout parses");
        let layout_sha256 = luxel_native_graphics_contract::sha256_prefixed(
            &serde_json::to_vec(&layout).expect("layout serializes"),
        );
        let julia = std::env::var_os("LUXEL_JULIA").unwrap_or_else(|| "julia".into());
        let (fields, provenance) = luxel_reference_runtime::run_julia_field_worker(
            &layout,
            &layout_sha256,
            &PathBuf::from(julia),
            &root.join("terrain_lab"),
        )
        .expect("Julia fields build without a graphics worker");
        let world = luxel_reference_runtime::build_world(layout, layout_sha256, fields, provenance)
            .expect("world artifact builds without a graphics worker");
        let packet = luxel_native_graphics_contract::lower_reference_world(&world)
            .expect("reference packet lowers");
        (packet, world)
    });
    (packet, world)
}

fn payload(packet: &GraphicsScenePacket, depth: f32) -> DepthCapturePayload {
    let count = packet.body.capture.width_px as usize * packet.body.capture.height_px as usize;
    let bytes: Vec<u8> = (0..count).flat_map(|_| depth.to_le_bytes()).collect();
    DepthCapturePayload {
        schema: "luxel.view-depth/v1".into(),
        packet_sha256: packet.packet_sha256.clone(),
        capture_id: packet.body.capture.capture_id.clone(),
        width_px: packet.body.capture.width_px,
        height_px: packet.body.capture.height_px,
        depth_sha256: sha256_prefixed(&bytes),
        depth_base64: STANDARD.encode(bytes),
    }
}
#[test]
fn depth_bytes_bind_to_packet_dimensions_and_positive_camera_depth() {
    let (packet, _) = fixtures();
    let value = payload(packet, packet.body.camera.far_plane_m);
    let verified = VerifiedDepthCapture::verify(packet, value.clone()).unwrap();
    assert!(
        verified
            .meters()
            .iter()
            .all(|d| *d == packet.body.camera.far_plane_m)
    );
    assert_eq!(sha256_prefixed(verified.bytes()), value.depth_sha256);
    for mutation in 0..6 {
        let mut bad = value.clone();
        match mutation {
            0 => bad.packet_sha256 = "sha256:wrong".into(),
            1 => bad.capture_id.push_str("-other"),
            2 => bad.width_px += 1,
            3 => bad.depth_sha256.push('0'),
            4 => bad.depth_base64.truncate(8),
            _ => bad.depth_base64.push_str("AAAA"),
        }
        assert!(
            VerifiedDepthCapture::verify(packet, bad).is_err(),
            "mutation {mutation}"
        );
    }
    for bad in [
        f32::NAN,
        f32::INFINITY,
        -1.0,
        0.0,
        packet.body.camera.far_plane_m * 2.0,
    ] {
        assert!(VerifiedDepthCapture::verify(packet, payload(packet, bad)).is_err());
    }
}

#[test]
fn grounding_observes_touching_and_floating_bases_but_refuses_occluded_bases() {
    use luxel_native_graphics_contract::depth_capture::measure_grounding;
    use luxel_native_graphics_contract::{
        BufferReference, CameraProjection, InstanceImportance, InstancePacket, Transform3d,
        seal_scene_packet,
    };
    let (reference, _) = fixtures();
    let mut body = reference.body.clone();
    body.camera.position_xyz_m = [0.0, 10.0, 0.0];
    body.camera.forward_xyz = [0.0, -1.0, 0.0];
    body.camera.up_xyz = [0.0, 0.0, -1.0];
    body.camera.projection = CameraProjection::Orthographic { span_m: 8.0 };
    body.camera.near_plane_m = 0.1;
    body.camera.far_plane_m = 20.0;
    body.camera.width_px = 32;
    body.camera.height_px = 32;
    body.capture.width_px = 32;
    body.capture.height_px = 32;
    body.terrain.heights_m = BufferReference::inline_f32(
        "grounding-flat",
        vec![0.0; body.terrain.resolution * body.terrain.resolution],
    );
    let mut mesh = body.meshes[0].clone();
    mesh.positions_m = vec![[-0.1, 0.0, 0.0], [0.1, 0.0, 0.0], [0.0, 0.0, 0.1]];
    mesh.normals = vec![[0.0, 1.0, 0.0]; 3];
    mesh.uv0 = vec![[0.0, 0.0]; 3];
    mesh.tangents.clear();
    mesh.indices = vec![0, 2, 1];
    body.instances = [
        (-1.0, 0.0, "kit-rock-touching"),
        (1.0, 1.0, "kit-rock-floating"),
    ]
    .map(|(x, y, id)| InstancePacket {
        instance_id: id.into(),
        mesh_id: mesh.mesh_id.clone(),
        material_id: mesh.material_id.clone(),
        importance: InstanceImportance::Landmark,
        transform: Transform3d {
            translation_xyz_m: [x, y, 0.0],
            rotation_xyzw: [0.0, 0.0, 0.0, 1.0],
            scale_xyz: [1.0; 3],
        },
        variation: None,
    })
    .to_vec();
    body.meshes = vec![mesh];
    body.overlays.clear();
    let packet = seal_scene_packet(body).unwrap();
    let mut value = payload(&packet, 10.0);
    let bytes: Vec<u8> = (0..32)
        .flat_map(|_| (0..32).flat_map(|x| (if x < 16 { 10.0f32 } else { 9.0f32 }).to_le_bytes()))
        .collect();
    value.depth_sha256 = sha256_prefixed(&bytes);
    value.depth_base64 = STANDARD.encode(bytes);
    let depth = VerifiedDepthCapture::verify(&packet, value).unwrap();
    let color = vec![255; 32 * 32 * 4];
    let result = measure_grounding(&packet, &color, &depth).unwrap();
    assert_eq!(result.body.candidate_count, 2);
    assert_eq!(result.body.samples.len(), 2);
    assert_eq!(result.body.contact_fraction_bp, Some(5000));
    assert_eq!(result.body.samples[0].gap_mm, 0);
    assert_eq!(result.body.samples[1].gap_mm, 1000);
    let blocked = VerifiedDepthCapture::verify(&packet, payload(&packet, 5.0)).unwrap();
    let hidden = measure_grounding(&packet, &color, &blocked).unwrap();
    assert_eq!(hidden.body.unobserved_count, 2);
    assert_eq!(hidden.body.contact_fraction_bp, None);
    let mut other = packet.clone();
    other.packet_sha256.push('0');
    assert!(measure_grounding(&other, &color, &depth).is_err());
}
