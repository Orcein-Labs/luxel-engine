use luxel_native_graphics_contract::input_session::{
    ACTOR_MOVEMENT_FRAME_SCHEMA, ActorMovementSession, INPUT_SIM_TICK_DT_SECONDS, InputFrame,
    InputSample, camera_intent, ds3_map, frame_from_sample,
};
use luxel_native_graphics_contract::{
    InstanceImportance, Transform3d, canonical_json, lower_reference_world, seal_scene_packet,
    validate_scene_packet,
};
use luxel_reference_runtime::{
    KinematicInput, KinematicStepDisposition, PhysicsWorld, WorldArtifact, validate_world_artifact,
};

const WORLD_FIXTURE: &[u8] = include_bytes!("fixtures/riverwatch-movement-world.json");
const ACTOR_INSTANCE_ID: &str = "movement-player-proxy";

fn world_fixture() -> WorldArtifact {
    serde_json::from_slice(WORLD_FIXTURE).expect("checked-in Julia-produced artifact parses")
}

fn packet_with_actor(
    world: &WorldArtifact,
) -> (
    luxel_native_graphics_contract::GraphicsScenePacket,
    Transform3d,
) {
    let physics = PhysicsWorld::new(world).expect("real world artifact validates");
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
    let source_x_rotation_half_angle = 0.3_f32;
    actor.transform = Transform3d {
        translation_xyz_m: spawn.body.position_xyz_m.map(|value| value as f32),
        rotation_xyzw: [
            source_x_rotation_half_angle.sin(),
            0.0,
            0.0,
            source_x_rotation_half_angle.cos(),
        ],
        scale_xyz: [0.8, 1.7, 0.8],
    };
    let source_transform = actor.transform.clone();
    body.instances.push(actor);
    (
        seal_scene_packet(body).expect("packet with primitive actor seals"),
        source_transform,
    )
}

fn assert_camera_matches(session: &ActorMovementSession<'_>) {
    let last = session.trace().last().expect("movement step exists");
    let (position, forward) = camera_intent(
        session.player_state(),
        session.camera_yaw_rad(),
        session.camera_pitch_rad(),
    )
    .expect("follow camera derives from the post-step body");
    assert_eq!(last.movement_frame.camera_position_xyz_m, position);
    assert_eq!(last.movement_frame.camera_forward_xyz, forward);
}

#[test]
fn raw_input_moves_packet_bound_actor_camera_and_replays_on_real_world() {
    assert!(WORLD_FIXTURE.len() < 512 * 1024);
    let world = world_fixture();
    validate_world_artifact(&world).expect("checked-in world retains full provenance");
    assert_eq!(
        world.body.julia_provenance.worker_id,
        "terrain_lab/bin/luxel_reference_world_fields.jl"
    );

    let (packet, source_transform) = packet_with_actor(&world);
    let source_packet_sha256 = packet.packet_sha256.clone();
    let source_packet_bytes = canonical_json(&packet).expect("base packet serializes");
    let mut session = ActorMovementSession::new(&packet, &world, ACTOR_INSTANCE_ID)
        .expect("actor binds to validated packet and world");
    let physics = PhysicsWorld::new(&world).expect("physics oracle binds to the same world");
    let oracle_start = physics
        .initialize_player()
        .expect("physics oracle starts at the authored spawn");
    let oracle_first = physics
        .step(
            &oracle_start,
            KinematicInput {
                expected_tick: 1,
                velocity_xz_mps: [0.0, -4.0],
            },
        )
        .expect("independent S-key velocity advances the real physics world");

    let start_position = session.player_state().body.position_xyz_m;
    let mut move_sample = InputSample::new(0.0);
    move_sample.held_keys = vec![ds3_map::KEY_S];
    let first = session
        .advance_sample(&move_sample)
        .expect("one sample advances one tick")
        .clone();
    assert_eq!(first.schema_version, ACTOR_MOVEMENT_FRAME_SCHEMA);
    assert_eq!(first.tick, 1);
    assert_eq!(
        first.physics_disposition,
        KinematicStepDisposition::Advanced
    );
    assert_eq!(first.source_packet_sha256, source_packet_sha256);
    assert_eq!(first.world_artifact_id, world.artifact_id);
    assert_eq!(first.world_artifact_sha256, world.artifact_sha256);
    assert_eq!(first.actor_instance_id, ACTOR_INSTANCE_ID);
    assert_eq!(first.physics_receipt_sha256, oracle_first.receipt_sha256);
    assert_eq!(session.player_state(), &oracle_first.body.next_state);
    let mapped = frame_from_sample(1, &move_sample, INPUT_SIM_TICK_DT_SECONDS)
        .expect("raw sample maps to the recorded semantic frame");
    assert_eq!(session.trace()[0].input_frame, mapped);

    let after_move = session.player_state().body.position_xyz_m;
    assert_ne!(after_move, start_position);
    for index in 0..3 {
        let expected = f64::from(source_transform.translation_xyz_m[index]) + after_move[index]
            - start_position[index];
        assert!(
            (f64::from(first.actor_transform.translation_xyz_m[index]) - expected).abs() < 1e-5,
            "actor translation axis {index} follows the physics displacement"
        );
    }
    assert_eq!(
        first.actor_transform.rotation_xyzw,
        source_transform.rotation_xyzw
    );
    assert_eq!(first.actor_transform.scale_xyz, source_transform.scale_xyz);
    assert_camera_matches(&session);

    let mut look_sample = InputSample::new(33.0);
    look_sample.cursor = Some([0.6, 0.5]);
    let second = session
        .advance_sample(&look_sample)
        .expect("look-only sample advances one tick")
        .clone();
    assert_eq!(second.tick, 2);
    assert_eq!(session.player_state().body.position_xyz_m, after_move);
    let yaw = session.camera_yaw_rad();
    assert!(yaw > 0.0);
    let (sin_yaw, cos_yaw) = (yaw * 0.5).sin_cos();
    let [sin_source_x, _, _, cos_source_x] = source_transform.rotation_xyzw;
    let expected_rotation = [
        cos_yaw * sin_source_x,
        sin_yaw * cos_source_x,
        -sin_yaw * sin_source_x,
        cos_yaw * cos_source_x,
    ];
    for (index, expected) in expected_rotation.into_iter().enumerate() {
        assert!(
            (second.actor_transform.rotation_xyzw[index] - expected).abs() < 1e-6,
            "actor rotation axis {index} composes yaw before the authored X rotation"
        );
    }
    assert_camera_matches(&session);

    let mut rejected = false;
    for _ in 0..100 {
        let before_position = session.player_state().body.position_xyz_m;
        let before_transform = session
            .trace()
            .last()
            .expect("prior movement frame exists")
            .movement_frame
            .actor_transform
            .clone();
        let step = session
            .advance_sample(&move_sample)
            .expect("well-formed samples consume fixed ticks")
            .clone();
        if step.physics_disposition == KinematicStepDisposition::Rejected {
            assert_eq!(session.player_state().body.position_xyz_m, before_position);
            assert_eq!(
                step.actor_transform.translation_xyz_m,
                before_transform.translation_xyz_m
            );
            rejected = true;
            break;
        }
    }
    assert!(
        rejected,
        "the authored world boundary or slope rejects the sustained route"
    );
    assert_camera_matches(&session);

    let recorded_inputs: Vec<InputFrame> = session
        .trace()
        .iter()
        .map(|step| step.input_frame.clone())
        .collect();
    let final_trace = session.trace().to_vec();
    let final_digest = session.trace_digest();
    let mut replay = ActorMovementSession::new(&packet, &world, ACTOR_INSTANCE_ID)
        .expect("replay binds to the same source identities");
    replay
        .replay_semantic_frames(&recorded_inputs)
        .expect("recorded semantic frames replay");
    assert_eq!(replay.trace(), final_trace);
    assert_eq!(replay.trace_digest(), final_digest);

    let previous_state = replay.player_state().clone();
    let previous_trace_len = replay.trace().len();
    let previous_digest = replay.trace_digest();
    let previous_yaw = replay.camera_yaw_rad();
    let mut stale = InputFrame::new(previous_state.body.tick);
    stale.look_dx_rad = 0.4;
    assert!(replay.advance_frame(stale).is_err());
    let mut nonfinite = InputFrame::new(previous_state.body.tick + 1);
    nonfinite.move_x = f32::NAN;
    assert!(replay.advance_frame(nonfinite).is_err());
    let mut out_of_range_gamepad = InputSample::new(44.0);
    out_of_range_gamepad.gamepad_axes = vec![0.0, 0.0, 1.01];
    assert!(replay.advance_sample(&out_of_range_gamepad).is_err());
    assert_eq!(replay.player_state(), &previous_state);
    assert_eq!(replay.camera_yaw_rad(), previous_yaw);
    assert_eq!(replay.trace().len(), previous_trace_len);
    assert_eq!(replay.trace_digest(), previous_digest);
    assert_eq!(canonical_json(&packet).unwrap(), source_packet_bytes);
}

#[test]
fn actor_movement_rejects_detached_world_and_missing_actor_packets() {
    let world = world_fixture();
    validate_world_artifact(&world).expect("checked-in world retains full provenance");
    let (packet, _) = packet_with_actor(&world);

    let mut detached_body = packet.body.clone();
    detached_body.world_artifact_id.push_str("-detached");
    let detached = seal_scene_packet(detached_body).expect("detached packet is well-formed");
    validate_scene_packet(&detached).expect("detached packet remains valid in isolation");
    let error = ActorMovementSession::new(&detached, &world, ACTOR_INSTANCE_ID)
        .err()
        .expect("movement refuses a packet bound to another world identity");
    assert_eq!(error.code, "provenance");

    let mut missing_actor_body = packet.body.clone();
    missing_actor_body
        .instances
        .retain(|instance| instance.instance_id != ACTOR_INSTANCE_ID);
    let missing_actor =
        seal_scene_packet(missing_actor_body).expect("packet without actor remains well-formed");
    validate_scene_packet(&missing_actor).expect("missing-actor packet validates in isolation");
    let error = ActorMovementSession::new(&missing_actor, &world, ACTOR_INSTANCE_ID)
        .err()
        .expect("movement refuses a missing actor instance");
    assert_eq!(error.code, "provenance");
}
