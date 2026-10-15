//! C3.2 input-driven session seam: typed HID samples, a Rust-owned fixed-tick
//! kinematic simulation over the certified world, third-person camera intent,
//! and a digest-bound deterministic input trace.
//!
//! Authority model: the Julia worker samples raw HID state (keyboard, mouse
//! cursor, raw gamepad axes) while presenting and reports exactly one typed
//! [`InputSample`] per presented **frame**. Only this module turns samples into
//! gameplay meaning: the DS3-style button mapping, camera-space movement
//! intent, `PhysicsWorld::step()` integration at the shared fixed tick rate,
//! and the third-person camera that follows the body. The drive loop is
//! `present_frames_with_input` → one [`InputFrame`] per sample → re-aim camera.
//! The worker never derives semantics; Rust never touches HID devices. Every
//! applied step is a digest-bound `KinematicStepReceipt`, and every batch
//! appends to an input trace whose rolling digest makes a replay
//! byte-comparable.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use luxel_reference_runtime::{
    KinematicInput, KinematicState, KinematicStepDisposition, PhysicsWorld, REFERENCE_TICK_RATE_HZ,
    WorldArtifact,
};

use crate::action_map::{ActionMap, frame_from_sample_mapped};
use crate::{
    BufferPayload, CameraProjection, GraphicsScenePacket, GraphicsWorkerError,
    GraphicsWorkerSupervisor, PromotedFrame, TerrainPacket, Transform3d, canonical_json,
    session::{PresentedGraphicsSession, WindowFrameReport},
    validate_scene_packet,
};
use luxel_live_evidence_contract::{
    CertifiedSnapshotIdentity, LIVE_FRAME_SCHEMA, LiveEvidenceSession, LiveFrameAttestation,
    LiveFrameTelemetry, PacketIdentity, SNAPSHOT_RESULT_SCHEMA, SessionState, TierAResult,
    TierASnapshotRequest,
};

pub const INPUT_SAMPLE_SCHEMA: &str = "luxel.input-sample/v1";
pub const INPUT_FRAME_SCHEMA: &str = "luxel.input-frame/v1";
pub const INPUT_TRACE_SCHEMA: &str = "luxel.input-trace/v1";
pub const ACTOR_MOVEMENT_FRAME_SCHEMA: &str = "luxel.actor-movement-frame/v1";
pub const ACTOR_MOVEMENT_TRACE_SCHEMA: &str = "luxel.actor-movement-trace/v1";
pub const ACTOR_MOVEMENT_TRACE_DIGEST_DOMAIN: &[u8] = b"luxel.actor-movement-trace/v1\0";

/// Fixed simulated timestep the worker batches against. The worker is asked
/// for whole multiples of this interval, so sim time and present time stay
/// composable without drifting.
pub const INPUT_SIM_TICK_DT_MS: f64 = 1000.0 / (REFERENCE_TICK_RATE_HZ as f64);
pub const INPUT_SIM_TICK_DT_SECONDS: f32 = (INPUT_SIM_TICK_DT_MS / 1000.0) as f32;

fn fixed_tick_frame(
    expected_tick: u64,
    sample: &InputSample,
) -> Result<InputFrame, GraphicsWorkerError> {
    frame_from_sample(expected_tick, sample, INPUT_SIM_TICK_DT_SECONDS)
}

/// Raw HID state sampled by the worker. Every field is device-raw: the
/// semantic meaning (sprint, crouch, camera yaw) is applied only by Rust.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct InputSample {
    pub schema_version: String,
    /// Worker-side monotonic timestamp of the sample, in milliseconds.
    pub timestamp_ms: f64,
    /// GLFW key codes currently held, from `glfwGetKey` reporting PRESS.
    pub held_keys: Vec<i32>,
    /// Normalized cursor position within the window, `(0..1, 0..1)`; `None`
    /// when the window is unfocused or the cursor is disabled.
    pub cursor: Option<[f64; 2]>,
    /// Raw joystick axes for joystick 1 in GLFW order, or an empty vector.
    pub gamepad_axes: Vec<f32>,
    /// Raw joystick button states for joystick 1, or an empty vector.
    pub gamepad_buttons: Vec<bool>,
}

impl InputSample {
    pub fn new(timestamp_ms: f64) -> Self {
        Self {
            schema_version: INPUT_SAMPLE_SCHEMA.to_owned(),
            timestamp_ms,
            held_keys: Vec::new(),
            cursor: None,
            gamepad_axes: Vec::new(),
            gamepad_buttons: Vec::new(),
        }
    }

    pub fn validate(&self) -> Result<(), GraphicsWorkerError> {
        if self.schema_version != INPUT_SAMPLE_SCHEMA {
            return Err(GraphicsWorkerError::protocol(
                "input sample schema is not the supported version",
            ));
        }
        if !self.timestamp_ms.is_finite() {
            return Err(GraphicsWorkerError::protocol(
                "input sample timestamp must be finite",
            ));
        }
        for key in &self.held_keys {
            // Positive codes are GLFW keys; the reserved negative codes
            // -2..-1 are the adapter's mouse-button channel.
            let is_mouse_button = (-2..=-1).contains(key);
            if *key == 0 || (*key < 0 && !is_mouse_button) {
                return Err(GraphicsWorkerError::protocol(
                    "held key codes must be GLFW keys or the reserved mouse codes",
                ));
            }
        }
        for value in self.gamepad_axes.iter().copied() {
            if !value.is_finite() || !(-1.0..=1.0).contains(&value) {
                return Err(GraphicsWorkerError::protocol(
                    "gamepad axes must be finite and within [-1, 1]",
                ));
            }
        }
        if let Some(cursor) = self.cursor {
            for value in cursor {
                if !value.is_finite() || !(-1.0..=2.0).contains(&value) {
                    return Err(GraphicsWorkerError::protocol(
                        "cursor position must be finite and near the window",
                    ));
                }
            }
        }
        Ok(())
    }
}

/// Fail closed unless the worker returned one typed sample per presented frame
/// with strictly increasing timestamps.
pub fn accept_presented_samples(
    samples: Vec<InputSample>,
    frames_presented: u32,
) -> Result<Vec<InputSample>, GraphicsWorkerError> {
    if samples.len()
        != usize::try_from(frames_presented)
            .map_err(|_| GraphicsWorkerError::protocol("presented frame count is not a usize"))?
    {
        return Err(GraphicsWorkerError::protocol(format!(
            "worker returned {} input samples for {} presented frames",
            samples.len(),
            frames_presented
        )));
    }
    let mut previous_timestamp_ms = None;
    for sample in &samples {
        sample.validate()?;
        if let Some(previous) = previous_timestamp_ms
            && sample.timestamp_ms <= previous
        {
            return Err(GraphicsWorkerError::protocol(
                "worker input sample timestamps must increase with presented frames",
            ));
        }
        previous_timestamp_ms = Some(sample.timestamp_ms);
    }
    Ok(samples)
}

/// Semantic movement intent for one fixed tick, already DS3-shaped: camera-
/// relative planar movement, look deltas, and the discrete buttons the arena
/// demo binds. Sprint changes the movement scalar; the combat buttons are
/// carried as state for the gameplay kit and do not affect locomotion.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct InputFrame {
    pub schema_version: String,
    /// Sim tick this frame drives (`state.tick + 1`).
    pub expected_tick: u64,
    /// Movement in camera space: `x` = strafe right, `y` = forward.
    pub move_x: f32,
    pub move_y: f32,
    /// Look intent in radians: yaw (around world up) and pitch.
    pub look_dx_rad: f32,
    pub look_dy_rad: f32,
    pub sprint: bool,
    pub roll: bool,
    pub guard: bool,
    pub light_attack: bool,
    pub heavy_attack: bool,
    pub interact: bool,
}

impl InputFrame {
    pub fn new(expected_tick: u64) -> Self {
        Self {
            schema_version: INPUT_FRAME_SCHEMA.to_owned(),
            expected_tick,
            move_x: 0.0,
            move_y: 0.0,
            look_dx_rad: 0.0,
            look_dy_rad: 0.0,
            sprint: false,
            roll: false,
            guard: false,
            light_attack: false,
            heavy_attack: false,
            interact: false,
        }
    }

    pub(crate) fn validate(&self) -> Result<(), GraphicsWorkerError> {
        if self.schema_version != INPUT_FRAME_SCHEMA {
            return Err(GraphicsWorkerError::protocol(
                "input frame schema is not the supported version",
            ));
        }
        for value in [self.move_x, self.move_y, self.look_dx_rad, self.look_dy_rad] {
            if !value.is_finite() {
                return Err(GraphicsWorkerError::protocol(
                    "input frame fields must be finite",
                ));
            }
        }
        if self.move_x.abs() > 1.0 || self.move_y.abs() > 1.0 {
            return Err(GraphicsWorkerError::protocol(
                "movement axes are unit-scaled and must stay within [-1, 1]",
            ));
        }
        Ok(())
    }
}

/// DS3-style Xbox-layout keyboard mapping over GLFW key codes. The gamepad
/// mapping over raw axes/buttons mirrors the same layout.
pub mod ds3_map {
    /// GLFW key codes (US layout, DS3 Xbox-layout equivalents).
    pub const KEY_W: i32 = 87;
    pub const KEY_A: i32 = 65;
    pub const KEY_S: i32 = 83;
    pub const KEY_D: i32 = 68;
    pub const KEY_SHIFT: i32 = 340; // left shift
    pub const KEY_SPACE: i32 = 32; // roll/dodge
    pub const KEY_E: i32 = 69; // interact
    /// Guard is a hold on mouse-right in DS3; keyboard stand-in.
    pub const KEY_CTRL: i32 = 341; // left control
    /// GLFW mouse buttons reported through the same held-keys channel with
    /// these negative codes (GLFW key codes are positive).
    pub const MOUSE_LEFT: i32 = -1;
    pub const MOUSE_RIGHT: i32 = -2;

    /// GLFW gamepad axis indices.
    pub const AXIS_LEFT_X: usize = 0;
    pub const AXIS_LEFT_Y: usize = 1;
    pub const AXIS_RIGHT_X: usize = 2;
    pub const AXIS_RIGHT_Y: usize = 3;
    pub const AXIS_RIGHT_TRIGGER: usize = 5;

    /// Standard GLFW gamepad button indices (libglfw mapping order).
    pub const BUTTON_A: usize = 0; // roll
    pub const BUTTON_B: usize = 1; // sprint (hold)
    pub const BUTTON_X: usize = 2; // light attack
    pub const BUTTON_Y: usize = 3; // heavy attack
    pub const BUTTON_LEFT_BUMPER: usize = 4; // guard
    pub const BUTTON_RIGHT_BUMPER: usize = 5; // interact
}

const MOVE_SPEED_MPS: f32 = 4.0;
const SPRINT_MULTIPLIER: f32 = 1.8;
pub(crate) const MOUSE_YAW_RATE_RAD_PER_NORM: f32 = 6.5;
pub(crate) const MOUSE_PITCH_RATE_RAD_PER_NORM: f32 = 3.0;
pub(crate) const GAMEPAD_LOOK_RATE_RAD_PER_S: f32 = 3.4;
const GAMEPAD_DEADZONE: f32 = 0.15;
pub(crate) const CAMERA_MIN_HEIGHT_M: f32 = 0.5;
const CAMERA_DISTANCE_M: f32 = 5.0;
const CAMERA_SHOULDER_M: f32 = 0.9;

/// Apply the DS3-style keyboard mapping to one raw sample. Only the keys the
/// arena demo binds are interpreted; everything else is ignored, not guessed.
pub fn frame_from_keyboard(frame: &mut InputFrame, sample: &InputSample) {
    let held = |key: i32| sample.held_keys.contains(&key);
    let mut x = 0.0f32;
    let mut y = 0.0f32;
    if held(ds3_map::KEY_W) {
        y += 1.0;
    }
    if held(ds3_map::KEY_S) {
        y -= 1.0;
    }
    if held(ds3_map::KEY_D) {
        x += 1.0;
    }
    if held(ds3_map::KEY_A) {
        x -= 1.0;
    }
    let length = (x * x + y * y).sqrt();
    if length > 1.0 {
        x /= length;
        y /= length;
    }
    frame.move_x = x;
    frame.move_y = y;
    frame.sprint = held(ds3_map::KEY_SHIFT);
    frame.roll = held(ds3_map::KEY_SPACE);
    frame.guard = held(ds3_map::KEY_CTRL) || held(ds3_map::MOUSE_RIGHT);
    frame.light_attack = held(ds3_map::MOUSE_LEFT);
    frame.heavy_attack = held(ds3_map::KEY_SHIFT) && held(ds3_map::MOUSE_LEFT);
    frame.interact = held(ds3_map::KEY_E);
}

/// Merge the DS3-style gamepad mapping into one frame: an active stick axis
/// overrides the keyboard axis for that stick, and pressed buttons OR in. A
/// resting or deadzoned gamepad never cancels keyboard intent.
pub fn frame_from_gamepad(frame: &mut InputFrame, sample: &InputSample, batch_seconds: f32) {
    let axes = &sample.gamepad_axes;
    let buttons = &sample.gamepad_buttons;
    if axes.is_empty() && buttons.is_empty() {
        return;
    }
    let axis = |index: usize| -> f32 {
        let value = axes.get(index).copied().unwrap_or(0.0);
        let magnitude = value.abs();
        if magnitude <= GAMEPAD_DEADZONE {
            0.0
        } else {
            value.signum() * (magnitude - GAMEPAD_DEADZONE) / (1.0 - GAMEPAD_DEADZONE)
        }
    };
    let button = |index: usize| -> bool { buttons.get(index).copied().unwrap_or(false) };
    let stick_x = axis(ds3_map::AXIS_LEFT_X);
    let stick_y = axis(ds3_map::AXIS_LEFT_Y);
    if stick_x != 0.0 {
        frame.move_x = stick_x;
    }
    if stick_y != 0.0 {
        // GLFW stick Y is positive down; forward is stick up.
        frame.move_y = -stick_y;
    }
    frame.look_dx_rad += axis(ds3_map::AXIS_RIGHT_X) * GAMEPAD_LOOK_RATE_RAD_PER_S * batch_seconds;
    frame.look_dy_rad += axis(ds3_map::AXIS_RIGHT_Y) * GAMEPAD_LOOK_RATE_RAD_PER_S * batch_seconds;
    frame.sprint |= button(ds3_map::BUTTON_B);
    frame.roll |= button(ds3_map::BUTTON_A);
    frame.guard |= button(ds3_map::BUTTON_LEFT_BUMPER);
    frame.light_attack |= button(ds3_map::BUTTON_X);
    frame.heavy_attack |= button(ds3_map::BUTTON_Y);
    frame.interact |= button(ds3_map::BUTTON_RIGHT_BUMPER);
}

/// Turn a raw sample into a semantic frame for the given tick, merging
/// keyboard and gamepad (gamepad wins where both drive the same axis).
pub fn frame_from_sample(
    expected_tick: u64,
    sample: &InputSample,
    batch_seconds: f32,
) -> Result<InputFrame, GraphicsWorkerError> {
    sample.validate()?;
    let mut frame = InputFrame::new(expected_tick);
    frame_from_keyboard(&mut frame, sample);
    frame_from_gamepad(&mut frame, sample, batch_seconds);
    // Mouse look: cursor displacement from center drives yaw/pitch like a
    // raw-relative approximation until a true cursor-lock seam exists.
    if let Some([cx, cy]) = sample.cursor {
        let dx = (cx - 0.5) as f32;
        let dy = (cy - 0.5) as f32;
        frame.look_dx_rad += dx.abs() * MOUSE_YAW_RATE_RAD_PER_NORM * dx.signum();
        frame.look_dy_rad += dy.abs() * MOUSE_PITCH_RATE_RAD_PER_NORM * dy.signum();
    }
    frame.validate()?;
    Ok(frame)
}

/// Unit-norm heading basis from a yaw angle in the right-handed, Y-up world:
/// `forward = (sin yaw, cos yaw)` lies in the XZ plane, and `right` is
/// `up × forward` (east when facing north), so strafing and the camera
/// shoulder offset agree with the movement basis.
fn heading_basis(yaw_rad: f32) -> ([f64; 2], [f64; 2]) {
    let forward = [(yaw_rad.sin()) as f64, (yaw_rad.cos()) as f64];
    let right = [forward[1], -forward[0]];
    (forward, right)
}

/// Compose the planar velocity command for one tick: camera-relative movement
/// rotated into world space, sprint-scaled. Rejected moves never move the
/// body; the step seam consumes the tick atomically.
fn velocity_for(frame: &InputFrame, yaw_rad: f32) -> [f64; 2] {
    let (forward, right) = heading_basis(yaw_rad);
    let scalar = if frame.sprint {
        MOVE_SPEED_MPS * SPRINT_MULTIPLIER
    } else {
        MOVE_SPEED_MPS
    };
    let world_x = right[0] * frame.move_x as f64 + forward[0] * frame.move_y as f64;
    let world_z = right[1] * frame.move_x as f64 + forward[1] * frame.move_y as f64;
    let length = (world_x * world_x + world_z * world_z).sqrt() as f32;
    let scale = if length > 1.0 {
        scalar / length
    } else {
        scalar
    };
    [world_x * scale as f64, world_z * scale as f64]
}

/// Third-person camera intent: behind and slightly above the body origin,
/// shoulder offset to the right, looking along the view heading. Height is
/// clamped above the ground so the camera never dips below terrain level.
pub fn camera_intent(
    body: &KinematicState,
    yaw_rad: f32,
    pitch_rad: f32,
) -> Result<([f32; 3], [f32; 3]), GraphicsWorkerError> {
    let position = body.body.position_xyz_m;
    let (forward, right) = heading_basis(yaw_rad);
    let pitch = pitch_rad.clamp(-1.2, 1.2) as f64;
    let eye = [
        position[0] - forward[0] * CAMERA_DISTANCE_M as f64 + right[0] * CAMERA_SHOULDER_M as f64,
        (position[1] + 2.0 + (pitch.sin() * CAMERA_DISTANCE_M as f64))
            .max(position[1] + CAMERA_MIN_HEIGHT_M as f64),
        position[2] - forward[1] * CAMERA_DISTANCE_M as f64 + right[1] * CAMERA_SHOULDER_M as f64,
    ];
    // Look direction from eye toward a point above the body origin.
    let target = [position[0], position[1] + 1.5, position[2]];
    let look = [target[0] - eye[0], target[1] - eye[1], target[2] - eye[2]];
    let length = (look[0] * look[0] + look[1] * look[1] + look[2] * look[2]).sqrt();
    if length <= f64::EPSILON || !length.is_finite() {
        return Err(GraphicsWorkerError::protocol(
            "camera look direction degenerated",
        ));
    }
    Ok((
        [eye[0] as f32, eye[1] as f32, eye[2] as f32],
        [
            (look[0] / length) as f32,
            (look[1] / length) as f32,
            (look[2] / length) as f32,
        ],
    ))
}

const FIRST_PERSON_EYE_HEIGHT_M: f32 = 1.6;
const TOP_DOWN_HEIGHT_M: f32 = 24.0;
const TOP_DOWN_SPAN_M: f32 = 32.0;
const CAMERA_BOOM_COLLISION_STEPS: u32 = 16;

/// Play camera modes. Default third-person matches [`camera_intent`].
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CameraProfile {
    ThirdPerson,
    ThirdPersonCollision,
    FirstPerson,
    TopDown,
}

/// Pose produced by a camera profile. `up` and `projection` are `None` when
/// the packet's authored basis and projection should stay.
#[derive(Clone, Debug, PartialEq)]
pub struct CameraPose {
    pub position: [f32; 3],
    pub forward: [f32; 3],
    pub up: Option<[f32; 3]>,
    pub projection: Option<CameraProjection>,
}

/// Bilinear sample of the sealed packet heightfield. Off-map returns `None`.
pub fn terrain_height_at(terrain: &TerrainPacket, x: f32, z: f32) -> Option<f32> {
    let BufferPayload::F32(heights) = &terrain.heights_m.payload else {
        return None;
    };
    let n = terrain.resolution;
    if n < 2 || heights.len() != n * n {
        return None;
    }
    let half_width = terrain.width_m * 0.5;
    let half_length = terrain.length_m * 0.5;
    if x.abs() > half_width || z.abs() > half_length {
        return None;
    }
    let column = (x / terrain.width_m + 0.5) * (n - 1) as f32;
    let row = (0.5 - z / terrain.length_m) * (n - 1) as f32;
    let c0 = (column.floor() as usize).min(n - 2);
    let r0 = (row.floor() as usize).min(n - 2);
    let tc = column - c0 as f32;
    let tr = row - r0 as f32;
    let sample = |r: usize, c: usize| heights[r * n + c];
    let top = sample(r0, c0) * (1.0 - tc) + sample(r0, c0 + 1) * tc;
    let bottom = sample(r0 + 1, c0) * (1.0 - tc) + sample(r0 + 1, c0 + 1) * tc;
    Some(top * (1.0 - tr) + bottom * tr)
}

/// Profiled camera intent. `ThirdPerson` is [`camera_intent`] with packet up
/// and projection left unchanged.
pub fn camera_intent_for_profile(
    body: &KinematicState,
    yaw_rad: f32,
    pitch_rad: f32,
    profile: CameraProfile,
    terrain: Option<&TerrainPacket>,
) -> Result<CameraPose, GraphicsWorkerError> {
    match profile {
        CameraProfile::ThirdPerson => {
            let (position, forward) = camera_intent(body, yaw_rad, pitch_rad)?;
            Ok(CameraPose {
                position,
                forward,
                up: None,
                projection: None,
            })
        }
        CameraProfile::ThirdPersonCollision => {
            let (desired, forward) = camera_intent(body, yaw_rad, pitch_rad)?;
            let position = match terrain {
                Some(terrain) => collide_boom(body, desired, terrain),
                None => {
                    return Err(GraphicsWorkerError::protocol(
                        "third-person collision camera requires the packet heightfield",
                    ));
                }
            };
            let look = [
                (body.body.position_xyz_m[0] as f32) - position[0],
                (body.body.position_xyz_m[1] as f32 + 1.5) - position[1],
                (body.body.position_xyz_m[2] as f32) - position[2],
            ];
            let length = (look[0] * look[0] + look[1] * look[1] + look[2] * look[2]).sqrt();
            let forward = if length > f32::EPSILON && length.is_finite() {
                [look[0] / length, look[1] / length, look[2] / length]
            } else {
                forward
            };
            Ok(CameraPose {
                position,
                forward,
                up: None,
                projection: None,
            })
        }
        CameraProfile::FirstPerson => {
            let pitch = pitch_rad.clamp(-1.2, 1.2);
            let cos_pitch = pitch.cos();
            let forward = [
                yaw_rad.sin() * cos_pitch,
                pitch.sin(),
                yaw_rad.cos() * cos_pitch,
            ];
            let position = [
                body.body.position_xyz_m[0] as f32,
                body.body.position_xyz_m[1] as f32 + FIRST_PERSON_EYE_HEIGHT_M,
                body.body.position_xyz_m[2] as f32,
            ];
            Ok(CameraPose {
                position,
                forward,
                up: None,
                projection: None,
            })
        }
        CameraProfile::TopDown => {
            let position = [
                body.body.position_xyz_m[0] as f32,
                body.body.position_xyz_m[1] as f32 + TOP_DOWN_HEIGHT_M,
                body.body.position_xyz_m[2] as f32,
            ];
            Ok(CameraPose {
                position,
                forward: [0.0, -1.0, 0.0],
                up: Some([0.0, 0.0, 1.0]),
                projection: Some(CameraProjection::Orthographic {
                    span_m: TOP_DOWN_SPAN_M,
                }),
            })
        }
    }
}

fn collide_boom(body: &KinematicState, desired_eye: [f32; 3], terrain: &TerrainPacket) -> [f32; 3] {
    let target = [
        body.body.position_xyz_m[0] as f32,
        body.body.position_xyz_m[1] as f32 + 1.5,
        body.body.position_xyz_m[2] as f32,
    ];
    let clearance = CAMERA_MIN_HEIGHT_M;
    for step in 0..=CAMERA_BOOM_COLLISION_STEPS {
        let t = step as f32 / CAMERA_BOOM_COLLISION_STEPS as f32;
        let point = [
            desired_eye[0] + (target[0] - desired_eye[0]) * t,
            desired_eye[1] + (target[1] - desired_eye[1]) * t,
            desired_eye[2] + (target[2] - desired_eye[2]) * t,
        ];
        match terrain_height_at(terrain, point[0], point[2]) {
            Some(height) if point[1] >= height + clearance => {
                return [point[0], point[1].max(height + clearance), point[2]];
            }
            Some(_) => {}
            None => {}
        }
    }
    let height = terrain_height_at(terrain, target[0], target[2]).unwrap_or(target[1]);
    [target[0], target[1].max(height + clearance), target[2]]
}

/// One applied or rejected simulation step, recorded for replay comparison.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TraceStep {
    pub frame: InputFrame,
    pub disposition: String,
    pub position_after_xyz_m: [f64; 3],
}

/// Rolling input-trace digest. The digest chains frame bytes with the previous
/// digest, so a replay is byte-comparable against the registered trace.
pub const INPUT_TRACE_DIGEST_DOMAIN: &[u8] = b"luxel.input-trace/v1\0";

pub fn trace_digest(steps: &[TraceStep]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(INPUT_TRACE_DIGEST_DOMAIN);
    for step in steps {
        hasher.update(serde_json::to_vec(step).expect("trace step serialization cannot fail"));
    }
    format!("sha256:{}", hex(&hasher.finalize()))
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

struct ActorBinding {
    instance_id: String,
    source_transform: Transform3d,
    spawn_state: KinematicState,
}

/// An input-driven native session: the presented session keeps presenting,
/// and this layer owns the semantics that drive its camera each batch. The
/// world artifact stays owned by the caller and borrowed here, matching the
/// session contract's `capture_and_promote(supervisor, world)` pattern.
pub struct InputDrivenSession<'world> {
    session: PresentedGraphicsSession,
    physics: PhysicsWorld<'world>,
    world: &'world luxel_reference_runtime::WorldArtifact,
    player: KinematicState,
    yaw_rad: f32,
    pitch_rad: f32,
    trace: Vec<TraceStep>,
    trace_digest: String,
    actor: Option<ActorBinding>,
    camera_profile: CameraProfile,
    action_map: ActionMap,
    evidence: Option<PresentedEvidenceBinder>,
}

impl<'world> InputDrivenSession<'world> {
    /// Bind an input-driven layer onto a presented session over a validated
    /// world. The player starts at the authored spawn.
    pub fn new(
        session: PresentedGraphicsSession,
        world: &'world luxel_reference_runtime::WorldArtifact,
    ) -> Result<Self, GraphicsWorkerError> {
        session.validate_bound_world(world)?;
        let physics_world = PhysicsWorld::new(world).map_err(|error| {
            GraphicsWorkerError::provenance(format!("physics binding failed: {error}"))
        })?;
        let player = physics_world.initialize_player().map_err(|error| {
            GraphicsWorkerError::provenance(format!("player spawn failed: {error}"))
        })?;
        Ok(Self {
            session,
            physics: physics_world,
            world,
            player,
            yaw_rad: 0.0,
            pitch_rad: 0.0,
            trace: Vec::new(),
            trace_digest: trace_digest(&[]),
            actor: None,
            camera_profile: CameraProfile::ThirdPerson,
            action_map: ActionMap::ds3_default(),
            evidence: None,
        })
    }

    /// Bind one unique packet instance as the moving actor. Displacement is
    /// measured from the current body so this is called at spawn.
    pub fn bind_actor(&mut self, instance_id: &str) -> Result<(), GraphicsWorkerError> {
        let mut matches = self
            .session
            .base_packet()
            .body
            .instances
            .iter()
            .filter(|instance| instance.instance_id == instance_id);
        let actor = matches.next().ok_or_else(|| {
            GraphicsWorkerError::provenance(format!(
                "movement actor instance {instance_id} is missing from the base packet"
            ))
        })?;
        if matches.next().is_some() {
            return Err(GraphicsWorkerError::provenance(format!(
                "movement actor instance {instance_id} is ambiguous in the base packet"
            )));
        }
        self.actor = Some(ActorBinding {
            instance_id: instance_id.to_owned(),
            source_transform: actor.transform.clone(),
            spawn_state: self.player.clone(),
        });
        Ok(())
    }

    pub fn actor_instance_id(&self) -> Option<&str> {
        self.actor.as_ref().map(|actor| actor.instance_id.as_str())
    }

    pub fn set_camera_profile(
        &mut self,
        profile: CameraProfile,
    ) -> Result<(), GraphicsWorkerError> {
        self.camera_profile = profile;
        self.apply_camera_pose()?;
        Ok(())
    }

    pub fn camera_profile(&self) -> CameraProfile {
        self.camera_profile
    }

    pub fn set_action_map(&mut self, map: ActionMap) -> Result<(), GraphicsWorkerError> {
        map.validate()?;
        self.action_map = map;
        Ok(())
    }

    pub fn action_map(&self) -> &ActionMap {
        &self.action_map
    }

    pub fn bind_evidence(
        &mut self,
        binder: PresentedEvidenceBinder,
    ) -> Result<(), GraphicsWorkerError> {
        if binder.packet_sha256() != self.session.base_packet().packet_sha256 {
            return Err(GraphicsWorkerError::provenance(
                "evidence binder packet identity is detached from the presented session",
            ));
        }
        self.evidence = Some(binder);
        Ok(())
    }

    pub fn evidence(&self) -> Option<&PresentedEvidenceBinder> {
        self.evidence.as_ref()
    }

    pub fn evidence_mut(&mut self) -> Option<&mut PresentedEvidenceBinder> {
        self.evidence.as_mut()
    }

    pub fn player_state(&self) -> &KinematicState {
        &self.player
    }

    pub fn camera_yaw_rad(&self) -> f32 {
        self.yaw_rad
    }

    pub fn camera_pitch_rad(&self) -> f32 {
        self.pitch_rad
    }

    pub fn trace(&self) -> &[TraceStep] {
        &self.trace
    }

    pub fn trace_digest(&self) -> &str {
        &self.trace_digest
    }

    /// Advance the fixed-tick simulation by one semantic frame. A rejected
    /// step (collision, slope, boundary) still consumes the tick and is
    /// recorded with its disposition — the world rejects, the session adapts.
    pub fn apply_frame(&mut self, frame: InputFrame) -> Result<&TraceStep, GraphicsWorkerError> {
        frame.validate()?;
        let expected = self.player.body.tick.checked_add(1).ok_or_else(|| {
            GraphicsWorkerError::protocol("input simulation tick counter overflow")
        })?;
        if frame.expected_tick != expected {
            return Err(GraphicsWorkerError::protocol(format!(
                "input frame expects tick {}, simulation is at tick {}",
                frame.expected_tick, self.player.body.tick
            )));
        }
        // Stage camera state until the physics call succeeds. A rejected
        // collision step is a successful receipt and still commits look input.
        let yaw_rad = wrap_angle(self.yaw_rad + frame.look_dx_rad);
        let pitch_rad = (self.pitch_rad + frame.look_dy_rad).clamp(-1.2, 1.2);
        let velocity = velocity_for(&frame, yaw_rad);
        let input = KinematicInput {
            expected_tick: frame.expected_tick,
            velocity_xz_mps: velocity,
        };
        let receipt = self.physics.step(&self.player, input).map_err(|error| {
            GraphicsWorkerError::provenance(format!("kinematic step rejected: {error}"))
        })?;
        let next_player = receipt.body.next_state.clone();
        let actor_override = if let Some(actor) = &self.actor {
            Some((
                actor.instance_id.clone(),
                moved_actor_transform(
                    &actor.source_transform,
                    &actor.spawn_state,
                    &next_player,
                    yaw_rad,
                )?,
            ))
        } else {
            None
        };
        self.push_camera_pose(&next_player, yaw_rad, pitch_rad)?;
        if let Some((instance_id, transform)) = actor_override {
            self.session
                .set_instance_override(&instance_id, transform)?;
        }
        self.yaw_rad = yaw_rad;
        self.pitch_rad = pitch_rad;
        self.player = next_player;
        let disposition = match receipt.body.disposition {
            luxel_reference_runtime::KinematicStepDisposition::Advanced => "advanced",
            luxel_reference_runtime::KinematicStepDisposition::Rejected => "rejected",
        };
        let step = TraceStep {
            frame,
            disposition: disposition.to_owned(),
            position_after_xyz_m: self.player.body.position_xyz_m,
        };
        self.trace.push(step);
        self.trace_digest = trace_digest(&self.trace);
        let last = self.trace.last().expect("a step was just pushed");
        Ok(last)
    }

    fn apply_camera_pose(&mut self) -> Result<(), GraphicsWorkerError> {
        self.push_camera_pose(&self.player.clone(), self.yaw_rad, self.pitch_rad)
    }

    fn push_camera_pose(
        &mut self,
        player: &KinematicState,
        yaw_rad: f32,
        pitch_rad: f32,
    ) -> Result<(), GraphicsWorkerError> {
        let terrain = &self.session.base_packet().body.terrain;
        let pose = camera_intent_for_profile(
            player,
            yaw_rad,
            pitch_rad,
            self.camera_profile,
            Some(terrain),
        )?;
        if pose.up.is_none() && pose.projection.is_none() {
            self.session.clear_camera_shape();
            self.session.set_camera(pose.position, pose.forward)?;
            return Ok(());
        }
        if let Some(up) = pose.up {
            self.session
                .set_camera_basis(pose.position, pose.forward, up)?;
        } else {
            self.session.set_camera(pose.position, pose.forward)?;
        }
        if let Some(projection) = pose.projection {
            self.session.set_camera_projection(projection)?;
        }
        Ok(())
    }

    /// Worker-free oracle of the drive loop: one raw sample becomes one fixed
    /// tick. The presented camera is re-aimed after each successful tick.
    pub fn apply_samples(&mut self, samples: &[InputSample]) -> Result<usize, GraphicsWorkerError> {
        for sample in samples {
            let expected_tick = self.player.body.tick.checked_add(1).ok_or_else(|| {
                GraphicsWorkerError::protocol("input simulation tick counter overflow")
            })?;
            let frame = frame_from_sample_mapped(
                expected_tick,
                sample,
                INPUT_SIM_TICK_DT_SECONDS,
                &self.action_map,
            )?;
            self.apply_frame(frame)?;
        }
        Ok(samples.len())
    }

    /// Replay recorded semantic frames from this session's current tick.
    /// A fresh session started on the same world reproduces a prior trace.
    pub fn replay_semantic_frames(
        &mut self,
        frames: &[InputFrame],
    ) -> Result<(), GraphicsWorkerError> {
        for frame in frames {
            self.apply_frame(frame.clone())?;
        }
        Ok(())
    }

    /// The drive loop presents only after the persistent window is open.
    pub fn require_open_window(&self) -> Result<(), GraphicsWorkerError> {
        if !self.session.is_window_open() {
            return Err(GraphicsWorkerError::protocol(
                "presented window is not open; open it before sampling input",
            ));
        }
        Ok(())
    }

    /// Present `target_frames` through the worker's HID sampler, then apply
    /// exactly one tick per returned sample. Does not invent or repeat samples
    /// from wall-clock elapsed time.
    pub fn sample_and_present(
        &mut self,
        supervisor: &mut GraphicsWorkerSupervisor,
        target_frames: u32,
    ) -> Result<(WindowFrameReport, usize), GraphicsWorkerError> {
        self.require_open_window()?;
        let (report, samples) = self
            .session
            .present_frames_with_input(supervisor, target_frames)?;
        let applied = self.apply_samples(&samples)?;
        if let Some(evidence) = self.evidence.as_mut() {
            let now_ms = evidence.advance_from_report(&report)?;
            evidence.observe_report(&report, self.session.base_packet(), now_ms)?;
        }
        Ok((report, applied))
    }

    /// Promote Tier-A evidence at the current camera through the offscreen
    /// authority path (unchanged contract: presented frames carry none).
    pub fn capture_and_promote(
        &mut self,
        supervisor: &mut GraphicsWorkerSupervisor,
    ) -> Result<crate::PromotedFrame, GraphicsWorkerError> {
        self.session.capture_and_promote(supervisor, self.world)
    }

    pub fn open_window(
        &mut self,
        supervisor: &mut GraphicsWorkerSupervisor,
    ) -> Result<crate::session::PresentedSessionOpenReceipt, GraphicsWorkerError> {
        self.session.open_window(supervisor)
    }

    pub fn close_window(
        &mut self,
        supervisor: &mut GraphicsWorkerSupervisor,
    ) -> Result<u64, GraphicsWorkerError> {
        self.session.close_window(supervisor)
    }

    pub fn presented_session(&self) -> &PresentedGraphicsSession {
        &self.session
    }

    pub fn is_window_open(&self) -> bool {
        self.session.is_window_open()
    }

    /// Promote a due Tier A sample through the offscreen authority path and
    /// complete it against the live (base) packet identity.
    pub fn sample_tier_a_if_due(
        &mut self,
        supervisor: &mut GraphicsWorkerSupervisor,
        now_ms: u64,
    ) -> Result<Option<PromotedFrame>, GraphicsWorkerError> {
        let Some(evidence) = self.evidence.as_mut() else {
            return Ok(None);
        };
        let Some(request) = evidence.tick(now_ms)? else {
            return Ok(None);
        };
        let promoted = self.session.capture_and_promote(supervisor, self.world)?;
        evidence.complete_promoted(&request, &promoted, now_ms)?;
        Ok(Some(promoted))
    }
}

/// Play session: the Star 1 drive loop plus a bound actor whose transform is
/// pushed as a presented instance override each tick.
pub struct PresentedPlaySession<'world> {
    inner: InputDrivenSession<'world>,
}

impl<'world> PresentedPlaySession<'world> {
    pub fn new(
        session: PresentedGraphicsSession,
        world: &'world WorldArtifact,
        actor_instance_id: &str,
    ) -> Result<Self, GraphicsWorkerError> {
        let mut inner = InputDrivenSession::new(session, world)?;
        inner.bind_actor(actor_instance_id)?;
        Ok(Self { inner })
    }

    pub fn inner(&self) -> &InputDrivenSession<'world> {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut InputDrivenSession<'world> {
        &mut self.inner
    }

    pub fn actor_transform(&self) -> Option<&Transform3d> {
        let id = self.inner.actor_instance_id()?;
        self.inner.session.instance_override(id)
    }

    pub fn apply_samples(&mut self, samples: &[InputSample]) -> Result<usize, GraphicsWorkerError> {
        self.inner.apply_samples(samples)
    }

    pub fn sample_and_present(
        &mut self,
        supervisor: &mut GraphicsWorkerSupervisor,
        target_frames: u32,
    ) -> Result<(WindowFrameReport, usize), GraphicsWorkerError> {
        self.inner.sample_and_present(supervisor, target_frames)
    }
}

/// Presented-path caller of [`LiveEvidenceSession`]. Records Tier B from a
/// window report and completes Tier A against the live packet identity.
pub struct PresentedEvidenceBinder {
    evidence: LiveEvidenceSession,
    project_id: String,
    session_id: String,
    packet_sha256: String,
    capabilities_sha256: String,
    last_observed_at_ms: u64,
}

impl PresentedEvidenceBinder {
    pub fn new(
        project_id: impl Into<String>,
        session_id: impl Into<String>,
        packet: &GraphicsScenePacket,
        capabilities_sha256: impl Into<String>,
        policy: luxel_live_evidence_contract::SamplingPolicy,
        started_at_ms: u64,
    ) -> Result<Self, GraphicsWorkerError> {
        let project_id = project_id.into();
        let session_id = session_id.into();
        let capabilities_sha256 = capabilities_sha256.into();
        let packet_identity = PacketIdentity {
            packet_sha256: packet.packet_sha256.clone(),
            capabilities_sha256: capabilities_sha256.clone(),
        };
        let evidence = LiveEvidenceSession::new(
            project_id.clone(),
            session_id.clone(),
            packet_identity,
            policy,
            started_at_ms,
        )
        .map_err(|error| {
            GraphicsWorkerError::protocol(format!("live evidence session rejected: {error}"))
        })?;
        Ok(Self {
            evidence,
            project_id,
            session_id,
            packet_sha256: packet.packet_sha256.clone(),
            capabilities_sha256,
            last_observed_at_ms: started_at_ms,
        })
    }

    pub fn state(&self) -> &SessionState {
        self.evidence.state()
    }

    pub fn packet_sha256(&self) -> &str {
        &self.packet_sha256
    }

    pub fn last_observed_at_ms(&self) -> u64 {
        self.last_observed_at_ms
    }

    pub fn pending_request(&self) -> Option<&TierASnapshotRequest> {
        self.evidence.pending_request()
    }

    /// Advance the binder clock by the presented batch duration. Does not
    /// read a wall clock.
    pub fn advance_from_report(
        &self,
        report: &WindowFrameReport,
    ) -> Result<u64, GraphicsWorkerError> {
        let elapsed_us = report.frame_times_us.iter().copied().sum::<u64>();
        let elapsed_ms = (elapsed_us / 1000).max(1);
        self.last_observed_at_ms
            .checked_add(elapsed_ms)
            .ok_or_else(|| GraphicsWorkerError::protocol("evidence clock overflow"))
    }

    pub fn observe_report(
        &mut self,
        report: &WindowFrameReport,
        packet: &GraphicsScenePacket,
        now_ms: u64,
    ) -> Result<(), GraphicsWorkerError> {
        if packet.packet_sha256 != self.packet_sha256 {
            return Err(GraphicsWorkerError::provenance(
                "live-frame packet identity is detached from the evidence session",
            ));
        }
        let frame_time_us = report
            .frame_time_us_at_percentile(50.0)
            .or_else(|| report.frame_times_us.last().copied())
            .unwrap_or(0);
        if frame_time_us == 0 {
            return Err(GraphicsWorkerError::protocol(
                "presented evidence requires a positive frame time",
            ));
        }
        let instance_count = u32::try_from(packet.body.instances.len()).map_err(|_| {
            GraphicsWorkerError::protocol("presented instance count exceeds Tier B range")
        })?;
        let attestation = LiveFrameAttestation {
            schema_version: LIVE_FRAME_SCHEMA.to_owned(),
            project_id: self.project_id.clone(),
            session_id: self.session_id.clone(),
            packet: PacketIdentity {
                packet_sha256: self.packet_sha256.clone(),
                capabilities_sha256: self.capabilities_sha256.clone(),
            },
            frame_sequence: report.window_presented_frames,
            observed_at_ms: now_ms,
            telemetry: LiveFrameTelemetry {
                frame_time_us,
                gpu_time_us: 0,
                draw_calls: report.frames_presented,
                submitted_instances: instance_count,
                visible_instances: instance_count,
                dropped_frames: 0,
            },
        };
        self.evidence
            .record_live_frame(attestation)
            .map_err(|error| {
                GraphicsWorkerError::protocol(format!("live-frame attestation rejected: {error}"))
            })?;
        self.last_observed_at_ms = now_ms;
        Ok(())
    }

    pub fn tick(
        &mut self,
        now_ms: u64,
    ) -> Result<Option<TierASnapshotRequest>, GraphicsWorkerError> {
        self.evidence.tick(now_ms).map_err(|error| {
            GraphicsWorkerError::protocol(format!("live evidence tick rejected: {error}"))
        })
    }

    pub fn complete_certified(
        &mut self,
        result: TierAResult,
        now_ms: u64,
    ) -> Result<(), GraphicsWorkerError> {
        self.evidence
            .complete_snapshot(Some(result), now_ms)
            .map_err(|error| {
                GraphicsWorkerError::protocol(format!("Tier A result rejected: {error}"))
            })?;
        self.last_observed_at_ms = now_ms;
        Ok(())
    }

    pub fn complete_promoted(
        &mut self,
        request: &TierASnapshotRequest,
        promoted: &PromotedFrame,
        now_ms: u64,
    ) -> Result<(), GraphicsWorkerError> {
        let capture_sha256 = promoted.frame.capture_sha256.clone();
        let result = TierAResult::Certified {
            schema_version: SNAPSHOT_RESULT_SCHEMA.to_owned(),
            request_id: request.request_id.clone(),
            project_id: self.project_id.clone(),
            session_id: self.session_id.clone(),
            packet: PacketIdentity {
                packet_sha256: self.packet_sha256.clone(),
                capabilities_sha256: self.capabilities_sha256.clone(),
            },
            completed_at_ms: now_ms,
            snapshot: CertifiedSnapshotIdentity {
                project_id: self.project_id.clone(),
                snapshot_id: promoted.frame.capture_id.clone(),
                candidate_sha256: capture_sha256,
                receipt_id: promoted.receipt.body.capture_id.clone(),
                receipt_sha256: promoted.receipt.receipt_sha256.clone(),
                validator_registry_sha256: promoted.receipt.body.renderer_identity_sha256.clone(),
                validator_id: "luxel.native-graphics/v1".to_owned(),
                receipt_schema: crate::FRAME_RECEIPT_SCHEMA.to_owned(),
            },
        };
        self.complete_certified(result, now_ms)
    }
}

/// A transient transform/camera override tied to an unchanged sealed scene
/// packet. This is CPU movement state; it does not claim that a renderer has
/// consumed the override.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ActorMovementFrame {
    pub schema_version: String,
    pub source_packet_sha256: String,
    pub world_artifact_id: String,
    pub world_artifact_sha256: String,
    pub actor_instance_id: String,
    pub tick: u64,
    pub actor_transform: Transform3d,
    pub camera_position_xyz_m: [f32; 3],
    pub camera_forward_xyz: [f32; 3],
    pub physics_disposition: KinematicStepDisposition,
    pub physics_receipt_sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ActorMovementTraceStep {
    pub schema_version: String,
    pub input_frame: InputFrame,
    pub movement_frame: ActorMovementFrame,
}

/// CPU-only movement playback that binds one actor transform to a validated
/// base packet and world. Packet identity and actor transform are snapshotted;
/// the session does not retain the packet or re-seal it.
pub struct ActorMovementSession<'world> {
    source_packet_sha256: String,
    world_artifact_id: String,
    world_artifact_sha256: String,
    actor_instance_id: String,
    actor_source_transform: Transform3d,
    spawn_state: KinematicState,
    player: KinematicState,
    physics: PhysicsWorld<'world>,
    yaw_rad: f32,
    pitch_rad: f32,
    trace: Vec<ActorMovementTraceStep>,
    trace_digest: [u8; 32],
}

impl<'world> ActorMovementSession<'world> {
    pub fn new(
        packet: &GraphicsScenePacket,
        world: &'world WorldArtifact,
        actor_instance_id: &str,
    ) -> Result<Self, GraphicsWorkerError> {
        validate_scene_packet(packet).map_err(GraphicsWorkerError::contract)?;
        let physics = PhysicsWorld::new(world).map_err(|error| {
            GraphicsWorkerError::provenance(format!("physics binding failed: {error}"))
        })?;
        if packet.body.world_artifact_id != world.artifact_id
            || packet.body.world_artifact_sha256 != world.artifact_sha256
            || packet.body.spatial_fields_sha256 != world.body.fields.spatial_sha256
        {
            return Err(GraphicsWorkerError::provenance(
                "movement packet provenance is detached from the validated world artifact",
            ));
        }
        let mut matches = packet
            .body
            .instances
            .iter()
            .filter(|instance| instance.instance_id == actor_instance_id);
        let actor = matches.next().ok_or_else(|| {
            GraphicsWorkerError::provenance(format!(
                "movement actor instance {actor_instance_id} is missing from the base packet"
            ))
        })?;
        if matches.next().is_some() {
            return Err(GraphicsWorkerError::provenance(format!(
                "movement actor instance {actor_instance_id} is ambiguous in the base packet"
            )));
        }
        let player = physics.initialize_player().map_err(|error| {
            GraphicsWorkerError::provenance(format!("player spawn failed: {error}"))
        })?;
        let trace_digest = Sha256::digest(ACTOR_MOVEMENT_TRACE_DIGEST_DOMAIN).into();
        Ok(Self {
            source_packet_sha256: packet.packet_sha256.clone(),
            world_artifact_id: world.artifact_id.clone(),
            world_artifact_sha256: world.artifact_sha256.clone(),
            actor_instance_id: actor_instance_id.to_owned(),
            actor_source_transform: actor.transform.clone(),
            spawn_state: player.clone(),
            player,
            physics,
            yaw_rad: 0.0,
            pitch_rad: 0.0,
            trace: Vec::new(),
            trace_digest,
        })
    }

    pub fn actor_instance_id(&self) -> &str {
        &self.actor_instance_id
    }

    pub fn player_state(&self) -> &KinematicState {
        &self.player
    }

    pub fn camera_yaw_rad(&self) -> f32 {
        self.yaw_rad
    }

    pub fn camera_pitch_rad(&self) -> f32 {
        self.pitch_rad
    }

    pub fn trace(&self) -> &[ActorMovementTraceStep] {
        &self.trace
    }

    pub fn trace_digest(&self) -> String {
        format!("sha256:{}", hex(&self.trace_digest))
    }

    /// Convert one raw sample to one fixed simulation tick; wall-clock timing
    /// and presentation batching are intentionally outside this CPU oracle.
    pub fn advance_sample(
        &mut self,
        sample: &InputSample,
    ) -> Result<&ActorMovementFrame, GraphicsWorkerError> {
        let expected_tick =
            self.player.body.tick.checked_add(1).ok_or_else(|| {
                GraphicsWorkerError::protocol("actor movement tick counter overflow")
            })?;
        let frame = fixed_tick_frame(expected_tick, sample)?;
        self.advance_frame(frame)
    }

    pub fn advance_frame(
        &mut self,
        frame: InputFrame,
    ) -> Result<&ActorMovementFrame, GraphicsWorkerError> {
        frame.validate()?;
        let expected_tick =
            self.player.body.tick.checked_add(1).ok_or_else(|| {
                GraphicsWorkerError::protocol("actor movement tick counter overflow")
            })?;
        if frame.expected_tick != expected_tick {
            return Err(GraphicsWorkerError::protocol(format!(
                "input frame expects tick {}, simulation is at tick {}",
                frame.expected_tick, self.player.body.tick
            )));
        }

        let yaw_rad = wrap_angle(self.yaw_rad + frame.look_dx_rad);
        let pitch_rad = (self.pitch_rad + frame.look_dy_rad).clamp(-1.2, 1.2);
        let input = KinematicInput {
            expected_tick: frame.expected_tick,
            velocity_xz_mps: velocity_for(&frame, yaw_rad),
        };
        let receipt = self.physics.step(&self.player, input).map_err(|error| {
            GraphicsWorkerError::provenance(format!("kinematic step rejected: {error}"))
        })?;
        let player = receipt.body.next_state.clone();
        let actor_transform = moved_actor_transform(
            &self.actor_source_transform,
            &self.spawn_state,
            &player,
            yaw_rad,
        )?;
        let (camera_position_xyz_m, camera_forward_xyz) =
            camera_intent(&player, yaw_rad, pitch_rad)?;
        let movement_frame = ActorMovementFrame {
            schema_version: ACTOR_MOVEMENT_FRAME_SCHEMA.to_owned(),
            source_packet_sha256: self.source_packet_sha256.clone(),
            world_artifact_id: self.world_artifact_id.clone(),
            world_artifact_sha256: self.world_artifact_sha256.clone(),
            actor_instance_id: self.actor_instance_id.clone(),
            tick: player.body.tick,
            actor_transform,
            camera_position_xyz_m,
            camera_forward_xyz,
            physics_disposition: receipt.body.disposition,
            physics_receipt_sha256: receipt.receipt_sha256,
        };
        let trace_step = ActorMovementTraceStep {
            schema_version: ACTOR_MOVEMENT_TRACE_SCHEMA.to_owned(),
            input_frame: frame,
            movement_frame,
        };
        let trace_digest = actor_movement_digest_step(&self.trace_digest, &trace_step)?;

        self.player = player;
        self.yaw_rad = yaw_rad;
        self.pitch_rad = pitch_rad;
        self.trace.push(trace_step);
        self.trace_digest = trace_digest;
        Ok(&self
            .trace
            .last()
            .expect("movement trace just appended")
            .movement_frame)
    }

    pub fn replay_semantic_frames(
        &mut self,
        frames: &[InputFrame],
    ) -> Result<(), GraphicsWorkerError> {
        for frame in frames {
            self.advance_frame(frame.clone())?;
        }
        Ok(())
    }
}

fn moved_actor_transform(
    source: &Transform3d,
    spawn: &KinematicState,
    player: &KinematicState,
    yaw_rad: f32,
) -> Result<Transform3d, GraphicsWorkerError> {
    let mut translation_xyz_m = [0.0; 3];
    for (index, translation) in translation_xyz_m.iter_mut().enumerate() {
        let delta = player.body.position_xyz_m[index] - spawn.body.position_xyz_m[index];
        let moved = f64::from(source.translation_xyz_m[index]) + delta;
        *translation = moved as f32;
        if !moved.is_finite() || !(*translation).is_finite() {
            return Err(GraphicsWorkerError::protocol(
                "actor movement produced a non-finite transform",
            ));
        }
    }
    let rotation_xyzw = if yaw_rad == 0.0 {
        source.rotation_xyzw
    } else {
        let (sin_yaw, cos_yaw) = (yaw_rad * 0.5).sin_cos();
        let [bx, by, bz, bw] = source.rotation_xyzw;
        let rotation = [
            cos_yaw * bx + sin_yaw * bz,
            cos_yaw * by + sin_yaw * bw,
            cos_yaw * bz - sin_yaw * bx,
            cos_yaw * bw - sin_yaw * by,
        ];
        let norm = rotation
            .iter()
            .map(|value| f64::from(*value) * f64::from(*value))
            .sum::<f64>()
            .sqrt();
        if !norm.is_finite() || norm <= f64::EPSILON {
            return Err(GraphicsWorkerError::protocol(
                "actor movement produced a degenerate rotation",
            ));
        }
        rotation.map(|value| (f64::from(value) / norm) as f32)
    };
    Ok(Transform3d {
        translation_xyz_m,
        rotation_xyzw,
        scale_xyz: source.scale_xyz,
    })
}

fn actor_movement_digest_step(
    previous: &[u8; 32],
    step: &ActorMovementTraceStep,
) -> Result<[u8; 32], GraphicsWorkerError> {
    let bytes = canonical_json(step).map_err(GraphicsWorkerError::contract)?;
    let mut hasher = Sha256::new();
    hasher.update(ACTOR_MOVEMENT_TRACE_DIGEST_DOMAIN);
    hasher.update(previous);
    hasher.update(bytes);
    Ok(hasher.finalize().into())
}

fn wrap_angle(angle: f32) -> f32 {
    let tau = std::f32::consts::TAU;
    let mut wrapped = angle % tau;
    if wrapped > std::f32::consts::PI {
        wrapped -= tau;
    } else if wrapped < -std::f32::consts::PI {
        wrapped += tau;
    }
    wrapped
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_keys(keys: &[i32]) -> InputSample {
        let mut sample = InputSample::new(100.0);
        sample.held_keys = keys.to_vec();
        sample
    }

    #[test]
    fn input_sample_rejects_bad_schema_and_nonfinite_fields() {
        let mut sample = InputSample::new(1.0);
        sample.schema_version = "luxel.input-sample/v0".into();
        assert!(sample.validate().is_err());
        let mut sample = InputSample::new(f64::NAN);
        sample.schema_version = INPUT_SAMPLE_SCHEMA.into();
        assert!(sample.validate().is_err());
        let mut sample = InputSample::new(1.0);
        sample.gamepad_axes = vec![f32::INFINITY];
        assert!(sample.validate().is_err());
        sample.gamepad_axes = vec![1.01];
        assert!(sample.validate().is_err());
        sample.gamepad_axes = vec![-1.01];
        assert!(sample.validate().is_err());
        sample.gamepad_axes = Vec::new();
        sample.cursor = Some([f64::NAN, 0.5]);
        assert!(sample.validate().is_err());
    }

    #[test]
    fn ds3_keyboard_mapping_matches_the_xbox_layout_shape() {
        let sample = sample_keys(&[ds3_map::KEY_W, ds3_map::KEY_SHIFT]);
        let frame = frame_from_sample(1, &sample, 1.0 / 30.0).expect("frame maps");
        assert!((frame.move_y - 1.0).abs() < 1e-6, "W drives forward");
        assert!(frame.sprint, "shift sprints");
        assert!(!frame.roll && !frame.guard, "unbound actions stay false");

        let sample = sample_keys(&[ds3_map::KEY_A, ds3_map::KEY_SPACE, ds3_map::MOUSE_RIGHT]);
        let frame = frame_from_sample(1, &sample, 1.0 / 30.0).expect("frame maps");
        assert!((frame.move_x + 1.0).abs() < 1e-6, "A strafes left");
        assert!(frame.roll, "space rolls");
        assert!(frame.guard, "right mouse guards");
    }

    #[test]
    fn ds3_gamepad_mapping_respects_deadzone_and_stick_sign() {
        let mut sample = InputSample::new(1.0);
        sample.gamepad_axes = vec![0.05, -1.0, 1.0, 0.0, 0.0, 0.0];
        sample.gamepad_buttons = vec![true, false, false, false, false, false];
        let frame = frame_from_sample(1, &sample, 1.0).expect("frame maps");
        assert_eq!(frame.move_x, 0.0, "small stick deflection is deadzoned");
        assert!((frame.move_y - 1.0).abs() < 1e-6, "stick up drives forward");
        assert!(frame.look_dx_rad > 0.0, "right stick right yaws positive");
        assert!(frame.roll, "A rolls");
    }

    #[test]
    fn fixed_tick_gamepad_look_uses_seconds() {
        let mut sample = InputSample::new(1.0);
        sample.gamepad_axes = vec![0.0, 0.0, 1.0, 0.0, 0.0, 0.0];
        let frame = fixed_tick_frame(1, &sample).expect("fixed-tick sample maps");
        let expected = GAMEPAD_LOOK_RATE_RAD_PER_S / REFERENCE_TICK_RATE_HZ as f32;
        assert!(
            (frame.look_dx_rad - expected).abs() < 1e-6,
            "one tick of full right-stick input is {expected} radians, got {}",
            frame.look_dx_rad
        );
    }

    #[test]
    fn movement_composes_in_camera_space() {
        let mut frame = InputFrame::new(1);
        frame.move_y = 1.0;
        let [vx, vz] = velocity_for(&frame, 0.0);
        assert!((vx - 0.0).abs() < 1e-9 && (vz - MOVE_SPEED_MPS as f64).abs() < 1e-9);
        // Yaw +90 degrees: forward becomes +x.
        let [vx, _] = velocity_for(&frame, std::f32::consts::FRAC_PI_2);
        assert!((vx - MOVE_SPEED_MPS as f64).abs() < 1e-6);
        // Diagonal movement is normalized before scaling.
        frame.move_x = 1.0;
        let [vx, vz] = velocity_for(&frame, 0.0);
        let speed = (vx * vx + vz * vz).sqrt();
        // f32 scale rounding admits ~1e-7 relative error at unit-normalized
        // diagonals; the tolerance matches that envelope, not exactness.
        assert!((speed - MOVE_SPEED_MPS as f64).abs() < 1e-6);
    }

    #[test]
    fn camera_intent_stays_above_the_body_origin() {
        let mut body_body = luxel_reference_runtime::KinematicStateBody {
            schema_version: luxel_reference_runtime::PHYSICS_STATE_SCHEMA.into(),
            world_artifact_id: "w".into(),
            world_artifact_sha256: "sha256:0".into(),
            tick: 3,
            position_xyz_m: [1.0, 6.0, 2.0],
            velocity_xz_mps: [0.0, 0.0],
        };
        body_body.position_xyz_m[1] = 6.0;
        let state = KinematicState {
            body: body_body,
            state_sha256: "sha256:0".into(),
        };
        let (position, forward) = camera_intent(&state, 0.0, -0.9).expect("intent resolves");
        assert!(f64::from(position[1]) > state.body.position_xyz_m[1]);
        let length =
            (forward[0] * forward[0] + forward[1] * forward[1] + forward[2] * forward[2]).sqrt();
        assert!((length - 1.0).abs() < 1e-5, "forward is normalized");
    }

    #[test]
    fn trace_digest_is_stable_and_order_sensitive() {
        let mut frame = InputFrame::new(1);
        frame.move_y = 0.5;
        let step = TraceStep {
            frame,
            disposition: "advanced".into(),
            position_after_xyz_m: [0.0, 0.0, 0.5],
        };
        let digest_a = trace_digest(std::slice::from_ref(&step));
        let digest_b = trace_digest(&[step]);
        assert_eq!(digest_a, digest_b, "digest is deterministic");
        let mut other = InputFrame::new(1);
        other.move_y = 0.4;
        let digest_c = trace_digest(&[TraceStep {
            frame: other,
            disposition: "advanced".into(),
            position_after_xyz_m: [0.0, 0.0, 0.4],
        }]);
        assert_ne!(digest_a, digest_c, "different traces digest differently");
        assert!(digest_a.starts_with("sha256:"));
    }

    #[test]
    fn frame_from_sample_rejects_out_of_range_movement() {
        let mut frame = InputFrame::new(1);
        frame.move_x = 1.5;
        assert!(frame.validate().is_err());
    }

    #[test]
    fn physics_failure_does_not_commit_staged_look_input() {
        let world: WorldArtifact = serde_json::from_str(include_str!(
            "../tests/fixtures/riverwatch-movement-world.json"
        ))
        .expect("checked-in Julia-produced world artifact parses");
        let packet = crate::lower_reference_world(&world).expect("world lowers to a packet");
        let request = crate::session::PresentedSessionRequest::new(
            "input-session-atomicity",
            packet.body.camera.width_px,
            packet.body.camera.height_px,
            false,
        )
        .expect("request is valid");
        let presented = PresentedGraphicsSession::new(&request, &packet, &world)
            .expect("validated world and packet bind");
        let mut session = InputDrivenSession::new(presented, &world).expect("input session binds");
        session.player.state_sha256 = "sha256:forged".into();
        let prior_player = session.player.clone();
        let prior_digest = session.trace_digest.clone();
        let mut frame = InputFrame::new(1);
        frame.look_dx_rad = 0.25;
        frame.look_dy_rad = -0.1;

        assert!(session.apply_frame(frame).is_err());
        assert_eq!(session.yaw_rad, 0.0);
        assert_eq!(session.pitch_rad, 0.0);
        assert_eq!(session.player, prior_player);
        assert!(session.trace.is_empty());
        assert_eq!(session.trace_digest, prior_digest);
    }

    #[test]
    fn accept_presented_samples_requires_one_monotonic_sample_per_frame() {
        let ok = vec![InputSample::new(1.0), InputSample::new(2.5)];
        let accepted =
            accept_presented_samples(ok, 2).expect("two increasing samples match two frames");
        assert_eq!(accepted.len(), 2);

        let count = accept_presented_samples(vec![InputSample::new(1.0)], 2)
            .expect_err("count mismatch is protocol");
        assert_eq!(count.code, "worker_protocol");

        let non_monotonic =
            accept_presented_samples(vec![InputSample::new(2.0), InputSample::new(2.0)], 2)
                .expect_err("equal timestamps are not increasing");
        assert_eq!(non_monotonic.code, "worker_protocol");

        let mut bad = InputSample::new(1.0);
        bad.gamepad_axes = vec![1.5];
        let invalid =
            accept_presented_samples(vec![bad], 1).expect_err("invalid sample fails closed");
        assert_eq!(invalid.code, "worker_protocol");
    }
}
