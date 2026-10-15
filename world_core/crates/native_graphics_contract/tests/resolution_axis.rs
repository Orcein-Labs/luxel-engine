use std::path::PathBuf;

use luxel_native_graphics_contract::{
    ADAPTER_REVISION, Campaign2Inputs, Campaign2LoweringInputs, Campaign2OutputResolution,
    Campaign2View, GraphicsScenePacket, GraphicsWorkerSupervisor, LAVA_REVISION, ParityContent,
    ParityPolicyCandidate, lower_campaign2_packet_inputs,
    lower_campaign2_packet_with_resolution_inputs, validate_scene_packet,
};
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

fn inputs<'a>() -> Campaign2Inputs<'a> {
    Campaign2Inputs {
        candidate: ParityPolicyCandidate::Null,
        content: ParityContent::default(),
        terrain_layers: None,
        kit: None,
        backdrop: None,
    }
}

#[test]
fn absent_and_explicit_native_resolution_keep_the_frozen_packet() {
    let (reference, _) = fixtures();
    let inputs = inputs();
    // Captured by running this fixture through the independent pre-R-2 source
    // at f8d10bf in a detached worktree.
    for (view, expected_digest, expected_size) in [
        (
            Campaign2View::Close,
            "sha256:911c804c99b4abf39aadea5bbc68819379dbab51210327d21d24baaf8c01a7e9",
            (768, 512),
        ),
        (
            Campaign2View::Medium,
            "sha256:84fdc9a7362bf01674eeac884a9e81afa0121143d31e99b0131223c1431529a7",
            (960, 640),
        ),
        (
            Campaign2View::Wide,
            "sha256:95536dc29f6685f7a9b5e5ae53ec9b466b4d72f50ab267ac2e7808637cdde6ed",
            (960, 640),
        ),
    ] {
        let absent =
            lower_campaign2_packet_inputs(reference, view, &inputs).expect("native lowering");
        let explicit = lower_campaign2_packet_with_resolution_inputs(
            reference,
            view,
            &Campaign2LoweringInputs {
                campaign: &inputs,
                output_resolution: Campaign2OutputResolution::Native,
            },
        )
        .expect("explicit native lowering");
        assert_eq!(
            absent, explicit,
            "{view:?} explicit native remains byte-identical"
        );
        assert_eq!(
            absent.packet_sha256, expected_digest,
            "{view:?} matches the independent pre-R-2 source build"
        );
        assert_eq!(
            (absent.body.camera.width_px, absent.body.camera.height_px),
            expected_size
        );
        assert_eq!(
            (absent.body.capture.width_px, absent.body.capture.height_px),
            expected_size
        );
    }
}

#[test]
fn scale_two_changes_only_dimensions_and_the_sealed_packet_digest() {
    let (reference, _) = fixtures();
    let inputs = inputs();
    for view in [
        Campaign2View::Close,
        Campaign2View::Medium,
        Campaign2View::Wide,
    ] {
        let native =
            lower_campaign2_packet_inputs(reference, view, &inputs).expect("native lowering");
        let scaled = lower_campaign2_packet_with_resolution_inputs(
            reference,
            view,
            &Campaign2LoweringInputs {
                campaign: &inputs,
                output_resolution: Campaign2OutputResolution::Scale2,
            },
        )
        .expect("scale 2 lowering");
        assert_eq!(scaled.body.camera.width_px, native.body.camera.width_px * 2);
        assert_eq!(
            scaled.body.camera.height_px,
            native.body.camera.height_px * 2
        );
        assert_eq!(scaled.body.capture.width_px, scaled.body.camera.width_px);
        assert_eq!(scaled.body.capture.height_px, scaled.body.camera.height_px);
        assert_eq!(
            u64::from(scaled.body.camera.width_px) * u64::from(native.body.camera.height_px),
            u64::from(native.body.camera.width_px) * u64::from(scaled.body.camera.height_px),
            "aspect ratio remains fixed"
        );
        assert_ne!(scaled.packet_sha256, native.packet_sha256);

        let mut normalized = scaled.body.clone();
        normalized.camera.width_px = native.body.camera.width_px;
        normalized.camera.height_px = native.body.camera.height_px;
        normalized.capture.width_px = native.body.capture.width_px;
        normalized.capture.height_px = native.body.capture.height_px;
        assert_eq!(
            normalized, native.body,
            "resolution leaves pose, content, and exposure intact"
        );
        validate_scene_packet(&scaled).expect("scaled packet validates");
    }
}

#[test]
fn capture_scale_parser_is_closed_and_fail_closed() {
    assert_eq!(
        Campaign2OutputResolution::parse_capture_scale("1").expect("native scale"),
        Campaign2OutputResolution::Native
    );
    assert_eq!(
        Campaign2OutputResolution::parse_capture_scale("2").expect("scale two"),
        Campaign2OutputResolution::Scale2
    );
    for bad in ["", "0", "3", "1.0", " 2", "2 ", "native"] {
        let error = Campaign2OutputResolution::parse_capture_scale(bad)
            .expect_err("reject unsupported scale");
        assert!(error.contains("exactly 1 or 2"), "{error}");
    }
}

#[test]
fn default_supervisor_authority_rejects_scaled_packet_before_render_dispatch() {
    use std::time::Duration;

    let (reference, world) = fixtures();
    let campaign = inputs();
    let lowered = lower_campaign2_packet_with_resolution_inputs(
        reference,
        Campaign2View::Close,
        &Campaign2LoweringInputs {
            campaign: &campaign,
            output_resolution: Campaign2OutputResolution::Scale2,
        },
    )
    .expect("scale 2 packet lowers");

    // A tiny framing-only worker proves supervisor authorization ordering
    // without initializing Lava or touching a GPU.
    let temp = std::env::temp_dir().join(format!("luxel-r2-authority-{}", std::process::id()));
    std::fs::create_dir_all(&temp).expect("temporary directory");
    let worker = temp.join("ready_only_worker.jl");
    let script = r#"
using SHA
function write_frame(payload)
    bytes = Vector{UInt8}(codeunits(payload))
    n = UInt32(length(bytes))
    write(stdout, UInt8[(n >> 24) & 0xff, (n >> 16) & 0xff, (n >> 8) & 0xff, n & 0xff])
    write(stdout, bytes)
    flush(stdout)
end
script_sha256 = bytes2hex(sha256(read(PROGRAM_FILE)))
payload = "{\"schema\":\"luxel.graphics-worker/v1\",\"kind\":\"ready\",\"script_sha256\":\"sha256:" * script_sha256 * "\",\"lava_revision\":\"__LAVA_REVISION__\",\"adapter_revision\":\"__ADAPTER_REVISION__\"}"
write_frame(payload)
sleep(30.0)
"#
    .replace("__LAVA_REVISION__", LAVA_REVISION)
    .replace("__ADAPTER_REVISION__", ADAPTER_REVISION);
    std::fs::write(&worker, script).expect("fake worker writes");
    let julia = std::env::var_os("LUXEL_JULIA").unwrap_or_else(|| "julia".into());
    let graphics_project = repo_root().join("graphics_lab");
    let mut supervisor = GraphicsWorkerSupervisor::start_with_timeouts(
        julia,
        graphics_project,
        &worker,
        Duration::from_secs(10),
        Duration::from_secs(2),
    )
    .expect("framing-only worker starts");

    let default_axis_error = supervisor
        .render_and_promote_campaign2(&lowered, world, &campaign)
        .expect_err("default Native authority rejects a Scale2 packet");
    assert_eq!(default_axis_error.code, "provenance");

    let resolution_inputs = Campaign2LoweringInputs {
        campaign: &campaign,
        output_resolution: Campaign2OutputResolution::Scale2,
    };
    let resolution_axis_error = supervisor
        .render_and_promote_campaign2_with_resolution(&lowered, world, &resolution_inputs)
        .expect_err("authorized Scale2 packet reaches capability gate");
    assert_eq!(
        resolution_axis_error.code, "worker_protocol",
        "{resolution_axis_error}"
    );
    assert!(
        resolution_axis_error
            .message
            .contains("capabilities must be validated")
    );
    let _ = std::fs::remove_dir_all(temp);
}
