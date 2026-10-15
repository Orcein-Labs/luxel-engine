//! Campaign B Stars 2–5: actor override, camera profiles, action map, evidence.

use luxel_live_evidence_contract::{
    CertifiedSnapshotIdentity, PacketIdentity, SNAPSHOT_RESULT_SCHEMA, SamplingPolicy,
    SessionState, TierAResult,
};
use luxel_native_graphics_contract::action_map::{
    ActionMap, InputBinding, frame_from_sample_mapped,
};
use luxel_native_graphics_contract::input_session::{
    CameraProfile, INPUT_SIM_TICK_DT_SECONDS, InputDrivenSession, InputSample,
    PresentedEvidenceBinder, PresentedPlaySession, camera_intent, camera_intent_for_profile,
    ds3_map, frame_from_sample, terrain_height_at,
};
use luxel_native_graphics_contract::session::{
    PresentedGraphicsSession, PresentedSessionRequest, WindowFrameReport,
};
use luxel_native_graphics_contract::{
    CameraProjection, InstanceImportance, Transform3d, lower_reference_world, seal_scene_packet,
    validate_scene_packet,
};
use luxel_reference_runtime::{WorldArtifact, validate_world_artifact};

const WORLD_FIXTURE: &[u8] = include_bytes!("fixtures/riverwatch-movement-world.json");
const ACTOR_INSTANCE_ID: &str = "movement-player-proxy";

fn world_fixture() -> WorldArtifact {
    serde_json::from_slice(WORLD_FIXTURE).expect("checked-in Julia-produced artifact parses")
}

fn packet_with_actor(world: &WorldArtifact) -> luxel_native_graphics_contract::GraphicsScenePacket {
    let physics = luxel_reference_runtime::PhysicsWorld::new(world).expect("world validates");
    let spawn = physics
        .initialize_player()
        .expect("authored player spawn resolves");
    let mut body = lower_reference_world(world)
        .expect("validated world lowers to a scene packet")
        .body;
    let mut actor = body
        .instances
        .iter()
        .find(|instance| instance.instance_id == "objective-beacon")
        .expect("lowered packet contains the primitive beacon mesh")
        .clone();
    actor.instance_id = ACTOR_INSTANCE_ID.into();
    actor.importance = InstanceImportance::GameplayCritical;
    let half = 0.3_f32;
    actor.transform = Transform3d {
        translation_xyz_m: spawn.body.position_xyz_m.map(|value| value as f32),
        rotation_xyzw: [half.sin(), 0.0, 0.0, half.cos()],
        scale_xyz: [0.8, 1.7, 0.8],
    };
    body.instances.push(actor);
    seal_scene_packet(body).expect("packet with primitive actor seals")
}

fn bind_play(world: &WorldArtifact) -> PresentedPlaySession<'_> {
    validate_world_artifact(world).expect("checked-in world retains full provenance");
    let packet = packet_with_actor(world);
    validate_scene_packet(&packet).expect("packet validates");
    let request = PresentedSessionRequest::new(
        "c3-2-playable-session",
        packet.body.camera.width_px,
        packet.body.camera.height_px,
        false,
    )
    .expect("request is valid");
    let presented = PresentedGraphicsSession::new(&request, &packet, world)
        .expect("validated world and packet bind");
    PresentedPlaySession::new(presented, world, ACTOR_INSTANCE_ID).expect("play session binds")
}

fn bind_input(world: &WorldArtifact) -> InputDrivenSession<'_> {
    let packet = lower_reference_world(world).expect("world lowers");
    let request = PresentedSessionRequest::new(
        "c3-2-profiles",
        packet.body.camera.width_px,
        packet.body.camera.height_px,
        false,
    )
    .expect("request is valid");
    let presented = PresentedGraphicsSession::new(&request, &packet, world).expect("session binds");
    InputDrivenSession::new(presented, world).expect("input session binds")
}

fn sample_keys(timestamp_ms: f64, keys: &[i32]) -> InputSample {
    let mut sample = InputSample::new(timestamp_ms);
    sample.held_keys = keys.to_vec();
    sample
}

fn sha_label(label: &str) -> String {
    luxel_native_graphics_contract::sha256_prefixed(label.as_bytes())
}

#[test]
fn play_session_writes_actor_override_matching_movement_sidecar() {
    let world = world_fixture();
    let packet = packet_with_actor(&world);
    let base_sha = packet.packet_sha256.clone();
    let mut play = bind_play(&world);
    let mut sidecar = luxel_native_graphics_contract::input_session::ActorMovementSession::new(
        play.inner().presented_session().base_packet(),
        &world,
        ACTOR_INSTANCE_ID,
    )
    .expect("sidecar binds the same actor");

    let sample = sample_keys(0.0, &[ds3_map::KEY_W]);
    play.apply_samples(&[sample.clone()])
        .expect("forward sample applies");
    let sidecar_frame = sidecar
        .advance_sample(&sample)
        .expect("sidecar advances")
        .clone();

    let override_transform = play
        .actor_transform()
        .expect("play session wrote an instance override")
        .clone();
    assert_eq!(override_transform, sidecar_frame.actor_transform);
    assert_eq!(
        play.inner().presented_session().base_packet().packet_sha256,
        base_sha,
        "presented base packet identity is unchanged"
    );

    let capture = play
        .inner()
        .presented_session()
        .capture_packet()
        .expect("capture packet reseals pose");
    assert_ne!(capture.packet_sha256, base_sha);
    let captured_actor = capture
        .body
        .instances
        .iter()
        .find(|instance| instance.instance_id == ACTOR_INSTANCE_ID)
        .expect("capture packet still names the actor");
    assert_eq!(captured_actor.transform, override_transform);

    let mut replay = bind_play(&world);
    replay
        .apply_samples(&[sample])
        .expect("replay applies the same sample");
    assert_eq!(replay.actor_transform(), Some(&override_transform));
}

#[test]
fn instance_override_fails_closed_for_unknown_ids() {
    let world = world_fixture();
    let mut session = bind_input(&world);
    let error = session
        .bind_actor("missing-actor")
        .expect_err("unknown instance fails closed");
    assert_eq!(error.code, "provenance");
    assert!(session.presented_session().instance_overrides().is_empty());
}

#[test]
fn camera_profiles_first_person_top_down_and_collision() {
    let world = world_fixture();
    let mut session = bind_input(&world);
    session
        .apply_samples(&[sample_keys(0.0, &[ds3_map::KEY_W])])
        .expect("body moves");
    let body = session.player_state().clone();
    let yaw = session.camera_yaw_rad();
    let pitch = session.camera_pitch_rad();
    let terrain = session
        .presented_session()
        .base_packet()
        .body
        .terrain
        .clone();

    let third = camera_intent_for_profile(
        &body,
        yaw,
        pitch,
        CameraProfile::ThirdPerson,
        Some(&terrain),
    )
    .expect("3P resolves");
    let (legacy_position, legacy_forward) = camera_intent(&body, yaw, pitch).expect("legacy 3P");
    assert_eq!(third.position, legacy_position);
    assert_eq!(third.forward, legacy_forward);
    assert!(third.projection.is_none());

    let first = camera_intent_for_profile(
        &body,
        yaw,
        pitch,
        CameraProfile::FirstPerson,
        Some(&terrain),
    )
    .expect("1P resolves");
    let dx = first.position[0] - body.body.position_xyz_m[0] as f32;
    let dz = first.position[2] - body.body.position_xyz_m[2] as f32;
    assert!(dx * dx + dz * dz < 1e-6, "first-person stays on the body");
    assert!((first.position[1] - body.body.position_xyz_m[1] as f32 - 1.6).abs() < 1e-4);

    let top = camera_intent_for_profile(&body, yaw, pitch, CameraProfile::TopDown, Some(&terrain))
        .expect("top-down resolves");
    assert_eq!(top.forward, [0.0, -1.0, 0.0]);
    assert_eq!(top.up, Some([0.0, 0.0, 1.0]));
    assert_eq!(
        top.projection,
        Some(CameraProjection::Orthographic { span_m: 32.0 })
    );

    session
        .set_camera_profile(CameraProfile::ThirdPersonCollision)
        .expect("collision profile applies");
    let eye = session.presented_session().camera_position();
    if let Some(height) = terrain_height_at(&terrain, eye[0], eye[2]) {
        assert!(
            eye[1] + 1e-4 >= height + 0.5,
            "collision boom stays above the heightfield"
        );
    }

    session
        .set_camera_profile(CameraProfile::TopDown)
        .expect("top-down applies");
    assert_eq!(
        session.presented_session().camera_projection(),
        CameraProjection::Orthographic { span_m: 32.0 }
    );
    session
        .set_camera_profile(CameraProfile::ThirdPerson)
        .expect("restore 3P");
    assert_eq!(
        session.presented_session().camera_projection(),
        session
            .presented_session()
            .base_packet()
            .body
            .camera
            .projection
    );
}

#[test]
fn action_map_ds3_default_matches_and_rebind_persist_invert_deadzone() {
    let sample = sample_keys(1.0, &[ds3_map::KEY_W, ds3_map::KEY_SHIFT]);
    let legacy = frame_from_sample(1, &sample, INPUT_SIM_TICK_DT_SECONDS).expect("legacy maps");
    let mapped = frame_from_sample_mapped(
        1,
        &sample,
        INPUT_SIM_TICK_DT_SECONDS,
        &ActionMap::ds3_default(),
    )
    .expect("mapped DS3");
    assert_eq!(mapped, legacy);

    let mut rebound = ActionMap::ds3_default();
    rebound.move_forward = vec![InputBinding::Key {
        code: ds3_map::KEY_E,
    }];
    let e_sample = sample_keys(1.0, &[ds3_map::KEY_E]);
    let e_frame = frame_from_sample_mapped(1, &e_sample, INPUT_SIM_TICK_DT_SECONDS, &rebound)
        .expect("rebind maps E to forward");
    assert!((e_frame.move_y - 1.0).abs() < 1e-6);
    let w_frame = frame_from_sample_mapped(1, &sample, INPUT_SIM_TICK_DT_SECONDS, &rebound)
        .expect("W is no longer forward");
    assert!(w_frame.move_y.abs() < 1e-6);

    let mut look = InputSample::new(2.0);
    look.cursor = Some([0.62, 0.5]);
    let default_look = frame_from_sample_mapped(
        1,
        &look,
        INPUT_SIM_TICK_DT_SECONDS,
        &ActionMap::ds3_default(),
    )
    .expect("look maps");
    let mut inverted = ActionMap::ds3_default();
    inverted.invert_look_x = true;
    let inverted_look =
        frame_from_sample_mapped(1, &look, INPUT_SIM_TICK_DT_SECONDS, &inverted).expect("invert");
    assert!((inverted_look.look_dx_rad + default_look.look_dx_rad).abs() < 1e-5);
    assert!(inverted_look.look_dx_rad < 0.0);

    let mut stick = InputSample::new(3.0);
    stick.gamepad_axes = vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
    stick.gamepad_axes[0] = 0.10;
    let dead = frame_from_sample_mapped(
        1,
        &stick,
        INPUT_SIM_TICK_DT_SECONDS,
        &ActionMap::ds3_default(),
    )
    .expect("default deadzone swallows 0.10");
    assert!(dead.move_x.abs() < 1e-6);
    let mut tight = ActionMap::ds3_default();
    tight.gamepad_deadzone = 0.05;
    let live = frame_from_sample_mapped(1, &stick, INPUT_SIM_TICK_DT_SECONDS, &tight)
        .expect("tighter deadzone lets 0.10 through");
    assert!(live.move_x > 0.0);

    let json = ActionMap::ds3_default().to_json().expect("map serializes");
    let restored = ActionMap::from_json(&json).expect("map deserializes");
    assert_eq!(restored, ActionMap::ds3_default());

    let mut bad = ActionMap::ds3_default();
    bad.gamepad_deadzone = 1.5;
    assert!(bad.validate().is_err());
}

#[test]
fn presented_evidence_records_live_frames_and_accepts_certified_samples() {
    let world = world_fixture();
    let packet = packet_with_actor(&world);
    let capabilities = sha_label("capabilities");
    let mut binder = PresentedEvidenceBinder::new(
        "riverwatch-play",
        "session-play",
        &packet,
        capabilities,
        SamplingPolicy {
            interval_ms: 100,
            sample_window_ms: 50,
        },
        10,
    )
    .expect("evidence binder starts");
    let report = WindowFrameReport {
        frames_presented: 1,
        window_presented_frames: 1,
        frame_times_us: vec![2_000],
    };
    binder
        .observe_report(&report, &packet, 20)
        .expect("typed present report records Tier B");
    assert!(matches!(binder.state(), SessionState::Live));

    let empty = WindowFrameReport {
        frames_presented: 1,
        window_presented_frames: 2,
        frame_times_us: vec![0],
    };
    assert!(binder.observe_report(&empty, &packet, 21).is_err());

    let request = binder.tick(110).expect("due tick").expect("sample is due");
    let result = TierAResult::Certified {
        schema_version: SNAPSHOT_RESULT_SCHEMA.to_owned(),
        request_id: request.request_id.clone(),
        project_id: request.project_id.clone(),
        session_id: request.session_id.clone(),
        packet: PacketIdentity {
            packet_sha256: packet.packet_sha256.clone(),
            capabilities_sha256: request.packet.capabilities_sha256.clone(),
        },
        completed_at_ms: 120,
        snapshot: CertifiedSnapshotIdentity {
            project_id: request.project_id.clone(),
            snapshot_id: "capture-play-1".into(),
            candidate_sha256: sha_label("candidate"),
            receipt_id: "receipt-play-1".into(),
            receipt_sha256: sha_label("receipt"),
            validator_registry_sha256: sha_label("registry"),
            validator_id: "luxel.native-graphics/v1".into(),
            receipt_schema: "luxel.graphics-frame-receipt/v1".into(),
        },
    };
    binder
        .complete_certified(result, 120)
        .expect("matching certified result is accepted");
    assert!(matches!(binder.state(), SessionState::Certified { .. }));
}
