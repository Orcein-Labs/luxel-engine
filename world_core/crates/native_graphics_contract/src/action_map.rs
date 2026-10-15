//! Rebindable DS3-shaped action map. Device-raw HID stays in [`InputSample`];
//! this module is the only place those samples become semantic buttons.

use serde::{Deserialize, Serialize};

use crate::GraphicsWorkerError;
use crate::input_session::{
    GAMEPAD_LOOK_RATE_RAD_PER_S, InputFrame, InputSample, MOUSE_PITCH_RATE_RAD_PER_NORM,
    MOUSE_YAW_RATE_RAD_PER_NORM, ds3_map,
};

pub const ACTION_MAP_SCHEMA: &str = "luxel.action-map/v1";
pub const DEFAULT_GAMEPAD_DEADZONE: f32 = 0.15;

/// One device-raw binding. Chorded DS3 heavy-attack stays a map flag rather
/// than a nested binding, so rebind JSON stays a flat list per action.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum InputBinding {
    Key { code: i32 },
    MouseButton { code: i32 },
    GamepadButton { index: u32 },
    GamepadAxis { index: u32, invert: bool },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ActionMap {
    pub schema_version: String,
    pub move_forward: Vec<InputBinding>,
    pub move_back: Vec<InputBinding>,
    pub strafe_left: Vec<InputBinding>,
    pub strafe_right: Vec<InputBinding>,
    pub sprint: Vec<InputBinding>,
    pub roll: Vec<InputBinding>,
    pub guard: Vec<InputBinding>,
    pub light_attack: Vec<InputBinding>,
    pub heavy_attack: Vec<InputBinding>,
    pub interact: Vec<InputBinding>,
    pub look_x: Vec<InputBinding>,
    pub look_y: Vec<InputBinding>,
    pub invert_look_x: bool,
    pub invert_look_y: bool,
    pub gamepad_deadzone: f32,
    /// DS3 keyboard: holding sprint and light attack is a heavy attack.
    pub keyboard_heavy_is_sprint_light: bool,
}

impl ActionMap {
    pub fn ds3_default() -> Self {
        Self {
            schema_version: ACTION_MAP_SCHEMA.to_owned(),
            move_forward: vec![
                InputBinding::Key {
                    code: ds3_map::KEY_W,
                },
                InputBinding::GamepadAxis {
                    index: ds3_map::AXIS_LEFT_Y as u32,
                    invert: true,
                },
            ],
            move_back: vec![InputBinding::Key {
                code: ds3_map::KEY_S,
            }],
            strafe_left: vec![InputBinding::Key {
                code: ds3_map::KEY_A,
            }],
            strafe_right: vec![
                InputBinding::Key {
                    code: ds3_map::KEY_D,
                },
                InputBinding::GamepadAxis {
                    index: ds3_map::AXIS_LEFT_X as u32,
                    invert: false,
                },
            ],
            sprint: vec![
                InputBinding::Key {
                    code: ds3_map::KEY_SHIFT,
                },
                InputBinding::GamepadButton {
                    index: ds3_map::BUTTON_B as u32,
                },
            ],
            roll: vec![
                InputBinding::Key {
                    code: ds3_map::KEY_SPACE,
                },
                InputBinding::GamepadButton {
                    index: ds3_map::BUTTON_A as u32,
                },
            ],
            guard: vec![
                InputBinding::Key {
                    code: ds3_map::KEY_CTRL,
                },
                InputBinding::MouseButton {
                    code: ds3_map::MOUSE_RIGHT,
                },
                InputBinding::GamepadButton {
                    index: ds3_map::BUTTON_LEFT_BUMPER as u32,
                },
            ],
            light_attack: vec![
                InputBinding::MouseButton {
                    code: ds3_map::MOUSE_LEFT,
                },
                InputBinding::GamepadButton {
                    index: ds3_map::BUTTON_X as u32,
                },
            ],
            heavy_attack: vec![InputBinding::GamepadButton {
                index: ds3_map::BUTTON_Y as u32,
            }],
            interact: vec![
                InputBinding::Key {
                    code: ds3_map::KEY_E,
                },
                InputBinding::GamepadButton {
                    index: ds3_map::BUTTON_RIGHT_BUMPER as u32,
                },
            ],
            look_x: vec![InputBinding::GamepadAxis {
                index: ds3_map::AXIS_RIGHT_X as u32,
                invert: false,
            }],
            look_y: vec![InputBinding::GamepadAxis {
                index: ds3_map::AXIS_RIGHT_Y as u32,
                invert: false,
            }],
            invert_look_x: false,
            invert_look_y: false,
            gamepad_deadzone: DEFAULT_GAMEPAD_DEADZONE,
            keyboard_heavy_is_sprint_light: true,
        }
    }

    pub fn validate(&self) -> Result<(), GraphicsWorkerError> {
        if self.schema_version != ACTION_MAP_SCHEMA {
            return Err(GraphicsWorkerError::protocol(
                "action map schema is not the supported version",
            ));
        }
        if !self.gamepad_deadzone.is_finite() || !(0.0..1.0).contains(&self.gamepad_deadzone) {
            return Err(GraphicsWorkerError::protocol(
                "action map gamepad deadzone must be finite in [0, 1)",
            ));
        }
        for binding in self.all_bindings() {
            binding.validate()?;
        }
        Ok(())
    }

    pub fn to_json(&self) -> Result<Vec<u8>, GraphicsWorkerError> {
        self.validate()?;
        serde_json::to_vec(self).map_err(|error| {
            GraphicsWorkerError::protocol(format!("action map serialization failed: {error}"))
        })
    }

    pub fn from_json(bytes: &[u8]) -> Result<Self, GraphicsWorkerError> {
        let map: Self = serde_json::from_slice(bytes).map_err(|error| {
            GraphicsWorkerError::protocol(format!("action map is not typed: {error}"))
        })?;
        map.validate()?;
        Ok(map)
    }

    fn all_bindings(&self) -> impl Iterator<Item = &InputBinding> {
        self.move_forward
            .iter()
            .chain(&self.move_back)
            .chain(&self.strafe_left)
            .chain(&self.strafe_right)
            .chain(&self.sprint)
            .chain(&self.roll)
            .chain(&self.guard)
            .chain(&self.light_attack)
            .chain(&self.heavy_attack)
            .chain(&self.interact)
            .chain(&self.look_x)
            .chain(&self.look_y)
    }

    fn digital_held(&self, sample: &InputSample, bindings: &[InputBinding]) -> bool {
        bindings.iter().any(|binding| match binding {
            InputBinding::Key { code } | InputBinding::MouseButton { code } => {
                sample.held_keys.contains(code)
            }
            InputBinding::GamepadButton { index } => sample
                .gamepad_buttons
                .get(*index as usize)
                .copied()
                .unwrap_or(false),
            InputBinding::GamepadAxis { .. } => false,
        })
    }

    fn axis_value(&self, sample: &InputSample, bindings: &[InputBinding]) -> f32 {
        let mut value = 0.0f32;
        for binding in bindings {
            if let InputBinding::GamepadAxis { index, invert } = binding {
                let raw = sample
                    .gamepad_axes
                    .get(*index as usize)
                    .copied()
                    .unwrap_or(0.0);
                let magnitude = raw.abs();
                if magnitude <= self.gamepad_deadzone {
                    continue;
                }
                let scaled = raw.signum() * (magnitude - self.gamepad_deadzone)
                    / (1.0 - self.gamepad_deadzone);
                let signed = if *invert { -scaled } else { scaled };
                if signed.abs() > value.abs() {
                    value = signed;
                }
            }
        }
        value
    }
}

impl InputBinding {
    fn validate(&self) -> Result<(), GraphicsWorkerError> {
        match self {
            Self::Key { code } => {
                if *code <= 0 {
                    return Err(GraphicsWorkerError::protocol(
                        "action map key codes must be positive GLFW keys",
                    ));
                }
            }
            Self::MouseButton { code } => {
                if !(-2..=-1).contains(code) {
                    return Err(GraphicsWorkerError::protocol(
                        "action map mouse buttons must use the reserved adapter codes",
                    ));
                }
            }
            Self::GamepadButton { index } => {
                if *index > 32 {
                    return Err(GraphicsWorkerError::protocol(
                        "action map gamepad button index is out of range",
                    ));
                }
            }
            Self::GamepadAxis { index, .. } => {
                if *index > 16 {
                    return Err(GraphicsWorkerError::protocol(
                        "action map gamepad axis index is out of range",
                    ));
                }
            }
        }
        Ok(())
    }
}

/// Turn a raw sample into a semantic frame using an explicit action map.
/// Mouse look from cursor-center remains implicit (PC relative approximation).
pub fn frame_from_sample_mapped(
    expected_tick: u64,
    sample: &InputSample,
    batch_seconds: f32,
    map: &ActionMap,
) -> Result<InputFrame, GraphicsWorkerError> {
    sample.validate()?;
    map.validate()?;
    let mut frame = InputFrame::new(expected_tick);
    let mut x = 0.0f32;
    let mut y = 0.0f32;
    if map.digital_held(sample, &map.strafe_right) {
        x += 1.0;
    }
    if map.digital_held(sample, &map.strafe_left) {
        x -= 1.0;
    }
    if map.digital_held(sample, &map.move_forward) {
        y += 1.0;
    }
    if map.digital_held(sample, &map.move_back) {
        y -= 1.0;
    }
    let length = (x * x + y * y).sqrt();
    if length > 1.0 {
        x /= length;
        y /= length;
    }
    frame.move_x = x;
    frame.move_y = y;
    let stick_x =
        map.axis_value(sample, &map.strafe_right) - map.axis_value(sample, &map.strafe_left);
    let stick_y =
        map.axis_value(sample, &map.move_forward) - map.axis_value(sample, &map.move_back);
    if stick_x != 0.0 {
        frame.move_x = stick_x.clamp(-1.0, 1.0);
    }
    if stick_y != 0.0 {
        frame.move_y = stick_y.clamp(-1.0, 1.0);
    }
    frame.sprint = map.digital_held(sample, &map.sprint);
    frame.roll = map.digital_held(sample, &map.roll);
    frame.guard = map.digital_held(sample, &map.guard);
    frame.light_attack = map.digital_held(sample, &map.light_attack);
    frame.heavy_attack = map.digital_held(sample, &map.heavy_attack);
    frame.interact = map.digital_held(sample, &map.interact);
    if map.keyboard_heavy_is_sprint_light
        && sample.held_keys.contains(&ds3_map::KEY_SHIFT)
        && sample.held_keys.contains(&ds3_map::MOUSE_LEFT)
    {
        frame.heavy_attack = true;
    }
    frame.look_dx_rad +=
        map.axis_value(sample, &map.look_x) * GAMEPAD_LOOK_RATE_RAD_PER_S * batch_seconds;
    frame.look_dy_rad +=
        map.axis_value(sample, &map.look_y) * GAMEPAD_LOOK_RATE_RAD_PER_S * batch_seconds;
    if let Some([cx, cy]) = sample.cursor {
        let dx = (cx - 0.5) as f32;
        let dy = (cy - 0.5) as f32;
        frame.look_dx_rad += dx.abs() * MOUSE_YAW_RATE_RAD_PER_NORM * dx.signum();
        frame.look_dy_rad += dy.abs() * MOUSE_PITCH_RATE_RAD_PER_NORM * dy.signum();
    }
    if map.invert_look_x {
        frame.look_dx_rad = -frame.look_dx_rad;
    }
    if map.invert_look_y {
        frame.look_dy_rad = -frame.look_dy_rad;
    }
    frame.validate()?;
    Ok(frame)
}
