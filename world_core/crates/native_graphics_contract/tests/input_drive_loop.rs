//! Campaign B Star 1: worker-sample drive loop on CPU.
//!
//! Uses the checked-in Julia-produced Riverwatch movement fixture. No window
//! and no graphics worker: `apply_samples` is the oracle for
//! `sample_and_present`.

use luxel_native_graphics_contract::input_session::{
    accept_presented_samples, camera_intent, ds3_map, InputDrivenSession, InputFrame, InputSample,
};
use luxel_native_graphics_contract::session::{PresentedGraphicsSession, PresentedSessionRequest};
use luxel_native_graphics_contract::{lower_reference_world, validate_scene_packet};
use luxel_reference_runtime::{validate_world_artifact, WorldArtifact};

const WORLD_FIXTURE: &[u8] = include_bytes!("fixtures/riverwatch-movement-world.json");

fn world_fixture() -> WorldArtifact {
    serde_json::from_slice(WORLD_FIXTURE).expect("checked-in Julia-produced artifact parses")
}

fn bind_session(world: &WorldArtifact) -> InputDrivenSession<'_> {
    validate_world_artifact(world).expect("checked-in world retains full provenance");
    let packet = lower_reference_world(world).expect("world lowers to a scene packet");
    validate_scene_packet(&packet).expect("lowered packet validates");
    let request = PresentedSessionRequest::new(
        "c3-2-drive-loop",
        packet.body.camera.width_px,
        packet.body.camera.height_px,
        false,
    )
    .expect("request is valid");
    let presented = PresentedGraphicsSession::new(&request, &packet, world)
        .expect("validated world and packet bind");
    InputDrivenSession::new(presented, world).expect("input session binds")
}

fn sample_keys(timestamp_ms: f64, keys: &[i32]) -> InputSample {
    let mut sample = InputSample::new(timestamp_ms);
    sample.held_keys = keys.to_vec();
    sample
}

#[test]
fn apply_samples_is_one_sample_one_tick_and_replays() {
    let world = world_fixture();
    let mut session = bind_session(&world);
    let spawn = session.player_state().clone();
    let spawn_digest = session.trace_digest().to_owned();

    let samples = [
        sample_keys(0.0, &[ds3_map::KEY_W]),
        sample_keys(33.0, &[ds3_map::KEY_D]),
        {
            let mut look = InputSample::new(66.0);
            look.cursor = Some([0.62, 0.48]);
            look
        },
    ];
    let applied = session
        .apply_samples(&samples)
        .expect("three samples apply three ticks");
    assert_eq!(applied, 3);
    assert_eq!(session.player_state().body.tick, 3);
    assert_eq!(session.trace().len(), 3);
    assert_ne!(
        session.player_state().body.position_xyz_m,
        spawn.body.position_xyz_m
    );
    assert!(session.camera_yaw_rad() != 0.0);
    let (camera_position, camera_forward) = camera_intent(
        session.player_state(),
        session.camera_yaw_rad(),
        session.camera_pitch_rad(),
    )
    .expect("follow camera derives from the post-step body");
    assert_eq!(
        session.presented_session().camera_position(),
        camera_position
    );
    assert_eq!(session.presented_session().camera_forward(), camera_forward);
    let live_trace = session.trace().to_vec();
    let live_digest = session.trace_digest().to_owned();
    let live_player = session.player_state().clone();
    let live_yaw = session.camera_yaw_rad();
    let live_pitch = session.camera_pitch_rad();

    let mut replay = bind_session(&world);
    let frames: Vec<InputFrame> = live_trace.iter().map(|step| step.frame.clone()).collect();
    replay
        .replay_semantic_frames(&frames)
        .expect("recorded semantic frames replay");
    assert_eq!(replay.trace(), live_trace);
    assert_eq!(replay.trace_digest(), live_digest);
    assert_eq!(replay.player_state(), &live_player);
    assert_eq!(replay.camera_yaw_rad(), live_yaw);
    assert_eq!(replay.camera_pitch_rad(), live_pitch);
    assert_eq!(
        replay.presented_session().camera_position(),
        camera_position
    );
    assert_eq!(replay.presented_session().camera_forward(), camera_forward);

    let mut once = bind_session(&world);
    assert_eq!(
        once.apply_samples(&[sample_keys(0.0, &[ds3_map::KEY_S])])
            .expect("one sample is one tick"),
        1
    );
    assert_eq!(once.player_state().body.tick, 1);
    assert_eq!(once.trace().len(), 1);

    let previous = replay.player_state().clone();
    let previous_len = replay.trace().len();
    let previous_digest = replay.trace_digest().to_owned();
    let previous_yaw = replay.camera_yaw_rad();
    let mut stale = InputFrame::new(previous.body.tick);
    stale.look_dx_rad = 0.4;
    assert!(replay.apply_frame(stale).is_err());
    assert_eq!(replay.player_state(), &previous);
    assert_eq!(replay.trace().len(), previous_len);
    assert_eq!(replay.trace_digest(), previous_digest);
    assert_eq!(replay.camera_yaw_rad(), previous_yaw);

    let closed = bind_session(&world);
    assert_eq!(closed.trace_digest(), spawn_digest);
    assert!(!closed.is_window_open());
}

#[test]
fn sample_and_present_fails_closed_without_an_open_window() {
    let world = world_fixture();
    let session = bind_session(&world);
    assert!(!session.is_window_open());
    let error = session
        .require_open_window()
        .expect_err("closed window cannot sample");
    assert_eq!(error.code, "worker_protocol");
    assert!(session.trace().is_empty());
}

#[test]
fn presented_sample_correspondence_is_independent_of_the_session() {
    let mismatch = accept_presented_samples(vec![InputSample::new(1.0), InputSample::new(2.0)], 3)
        .expect_err("three frames need three samples");
    assert_eq!(mismatch.code, "worker_protocol");
}
