use std::path::PathBuf;

use luxel_native_graphics_contract::{
    ADAPTER_REVISION, Campaign2FeatureInputs, Campaign2Features, Campaign2Inputs,
    Campaign2LoweringInputs, Campaign2OutputResolution, Campaign2View, ContactAoPolicy,
    GraphicsScenePacket, GraphicsWorkerSupervisor, LAVA_REVISION, ParityContent,
    ParityPolicyCandidate, ShadowCascadesPolicy, lower_campaign2_packet_with_features,
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
fn absent_features_preserve_all_frozen_views() {
    let (reference, _) = fixtures();
    let campaign = inputs();
    let resolution = Campaign2LoweringInputs {
        campaign: &campaign,
        output_resolution: Campaign2OutputResolution::Native,
    };
    for view in [
        Campaign2View::Close,
        Campaign2View::Medium,
        Campaign2View::Wide,
    ] {
        assert_eq!(
            lower_campaign2_packet_with_resolution_inputs(reference, view, &resolution).unwrap(),
            lower_campaign2_packet_with_features(
                reference,
                view,
                &Campaign2FeatureInputs {
                    resolution,
                    features: Campaign2Features::default()
                }
            )
            .unwrap()
        );
    }
}

#[test]
fn selected_policy_axes_are_sealed_and_do_not_change_scene_content() {
    let (reference, _) = fixtures();
    let campaign = inputs();
    let resolution = Campaign2LoweringInputs {
        campaign: &campaign,
        output_resolution: Campaign2OutputResolution::Scale2,
    };
    let base =
        lower_campaign2_packet_with_resolution_inputs(reference, Campaign2View::Wide, &resolution)
            .unwrap();
    let selected = lower_campaign2_packet_with_features(
        reference,
        Campaign2View::Wide,
        &Campaign2FeatureInputs {
            resolution,
            features: Campaign2Features {
                shadow_cascades: Some(ShadowCascadesPolicy::CONVERGE4),
                contact_ao: Some(ContactAoPolicy {
                    radius_cm: 80,
                    strength_bp: 3000,
                    bias_mm: 20,
                }),
                forest_profile: None,
            },
        },
    )
    .unwrap();
    validate_scene_packet(&selected).unwrap();
    assert_ne!(base.packet_sha256, selected.packet_sha256);
    let policy = selected.body.render_policy.as_ref().unwrap();
    assert_eq!(policy.shadow_fit, None);
    assert_eq!(
        policy.shadow_cascades,
        Some(ShadowCascadesPolicy::CONVERGE4)
    );
    let mut reset = selected.body;
    reset.render_policy = base.body.render_policy;
    assert_eq!(reset, base.body);
}

#[test]
fn supervisor_rejects_mismatched_features_before_dispatch() {
    use std::time::Duration;

    let (reference, world) = fixtures();
    let campaign = inputs();
    let feature_inputs = Campaign2FeatureInputs {
        resolution: Campaign2LoweringInputs {
            campaign: &campaign,
            output_resolution: Campaign2OutputResolution::Native,
        },
        features: Campaign2Features {
            contact_ao: Some(ContactAoPolicy {
                radius_cm: 80,
                strength_bp: 3000,
                bias_mm: 20,
            }),
            ..Campaign2Features::default()
        },
    };
    let lowered =
        lower_campaign2_packet_with_features(reference, Campaign2View::Close, &feature_inputs)
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
        .expect_err("default Native authority rejects a feature-enabled packet");
    assert_eq!(default_axis_error.code, "provenance");

    let resolution_axis_error = supervisor
        .render_and_promote_campaign2_with_features(&lowered, world, &feature_inputs)
        .expect_err("authorized feature-enabled packet reaches capability gate");
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

#[test]
fn cli_feature_values_fail_before_paths_or_worker_start() {
    let binary = env!("CARGO_BIN_EXE_luxel-native-graphics-contract");
    for name in [
        "LUXEL_SHADOW_CASCADES",
        "LUXEL_CONTACT_AO",
        "LUXEL_FOREST_PROFILE",
    ] {
        let output = std::process::Command::new(binary)
            .arg("render-campaign2-layout")
            .env_remove("LUXEL_CAPTURE_SCALE")
            .env_remove("LUXEL_SHADOW_CASCADES")
            .env_remove("LUXEL_CONTACT_AO")
            .env_remove("LUXEL_FOREST_PROFILE")
            .env(name, "unsupported")
            .output()
            .unwrap();
        assert!(!output.status.success());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains(name), "{error}");
        assert!(
            !error.contains("LAYOUT PATH"),
            "selection must fail before missing paths: {error}"
        );
    }
    for (name, value) in [
        ("LUXEL_SHADOW_CASCADES", "0"),
        ("LUXEL_SHADOW_CASCADES", "1"),
        ("LUXEL_CONTACT_AO", "0"),
        ("LUXEL_CONTACT_AO", "1"),
        ("LUXEL_FOREST_PROFILE", "none"),
        ("LUXEL_FOREST_PROFILE", "heartwood"),
    ] {
        let output = std::process::Command::new(binary)
            .arg("render-campaign2-layout")
            .env_remove("LUXEL_CAPTURE_SCALE")
            .env_remove("LUXEL_SHADOW_CASCADES")
            .env_remove("LUXEL_CONTACT_AO")
            .env_remove("LUXEL_FOREST_PROFILE")
            .env(name, value)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("LAYOUT PATH"));
    }
}

#[cfg(unix)]
#[test]
fn cli_non_utf8_feature_values_fail_closed() {
    use std::os::unix::ffi::OsStringExt;
    for name in [
        "LUXEL_SHADOW_CASCADES",
        "LUXEL_CONTACT_AO",
        "LUXEL_FOREST_PROFILE",
    ] {
        let output =
            std::process::Command::new(env!("CARGO_BIN_EXE_luxel-native-graphics-contract"))
                .arg("render-campaign2-layout")
                .env_remove("LUXEL_CAPTURE_SCALE")
                .env_remove("LUXEL_SHADOW_CASCADES")
                .env_remove("LUXEL_CONTACT_AO")
                .env_remove("LUXEL_FOREST_PROFILE")
                .env(name, std::ffi::OsString::from_vec(vec![255]))
                .output()
                .unwrap();
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains(name));
    }
}
