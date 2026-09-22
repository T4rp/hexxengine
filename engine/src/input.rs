use std::collections::{HashSet, VecDeque};

use glam::{Vec2, Vec3, Vec3Swizzles};
use rapier3d::parry::utils::hashmap::HashMap;
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta, RawKeyEvent},
    keyboard::{KeyCode, PhysicalKey},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputState {
    Pressed,
    Released,
    Changed,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputEvent {
    MouseButtonEvent {
        button: MouseButton,
        state: InputState,
        position: Vec2,
    },
    MouseScrollEvent {
        delta: f32,
    },
    KeyboardEvent {
        key: KeyCode,
        state: InputState,
    },
}

#[derive(Clone, Copy)]
pub struct MouseButtonEvent {
    pub button: MouseButton,
    pub state: InputState,
    pub position: Vec2,
}

struct InputEventRecord {
    key_states: HashMap<KeyCode, InputState>,
    scroll_delta: f32,
    left_mouse: Option<MouseButtonEvent>,
    middle_mouse: Option<MouseButtonEvent>,
    right_mouse: Option<MouseButtonEvent>,
}

impl InputEventRecord {
    fn new() -> Self {
        InputEventRecord {
            key_states: HashMap::default(),
            scroll_delta: 0.0,
            left_mouse: None,
            middle_mouse: None,
            right_mouse: None,
        }
    }

    fn clear(&mut self) {
        self.key_states.clear();
        self.scroll_delta = 0.0;
        self.left_mouse = None;
        self.middle_mouse = None;
        self.right_mouse = None;
    }
}

pub struct InputHandler {
    keys_down: HashSet<KeyCode>,
    pub right_mouse_down: bool,
    pub left_mouse_down: bool,

    pub mouse_position: Vec2,
    pub mouse_delta: Vec3,
    input_event_log: Vec<InputEvent>,
    input_event_record: InputEventRecord,
}

impl Default for InputHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl InputHandler {
    pub fn new() -> Self {
        Self {
            keys_down: HashSet::new(),
            right_mouse_down: false,
            left_mouse_down: false,
            mouse_position: Vec2::ZERO,
            mouse_delta: Vec3::ZERO,
            input_event_log: Vec::new(),
            input_event_record: InputEventRecord::new(),
        }
    }

    fn log_event(&mut self, input_event: InputEvent) {
        match input_event {
            InputEvent::MouseButtonEvent {
                button,
                state,
                position,
            } => {
                let mouse_input_event = MouseButtonEvent {
                    button,
                    state,
                    position,
                };

                match button {
                    MouseButton::Left => {
                        self.input_event_record.left_mouse = Some(mouse_input_event);
                    }
                    MouseButton::Right => {
                        self.input_event_record.right_mouse = Some(mouse_input_event);
                    }
                    MouseButton::Middle => {
                        self.input_event_record.middle_mouse = Some(mouse_input_event);
                    }
                    _ => {}
                };
            }
            InputEvent::MouseScrollEvent { delta } => {
                self.input_event_record.scroll_delta = delta;
            }
            InputEvent::KeyboardEvent { key, state } => {
                self.input_event_record.key_states.insert(key, state);
            }
        }

        self.input_event_log.push(input_event);
    }

    pub fn clear(&mut self) {
        self.mouse_delta = Vec3::ZERO;
        self.clear_events();
    }

    pub fn raw_key_input(&mut self, event: &RawKeyEvent) {
        let PhysicalKey::Code(key) = event.physical_key else {
            return;
        };

        match event.state {
            ElementState::Pressed => {
                self.keys_down.insert(key);
            }
            ElementState::Released => {
                self.keys_down.remove(&key);
            }
        }
    }

    pub fn key_input(&mut self, event: &KeyEvent) {
        let PhysicalKey::Code(key) = event.physical_key else {
            return;
        };

        match event.state {
            ElementState::Pressed => {
                self.keys_down.insert(key);
            }
            ElementState::Released => {
                self.keys_down.remove(&key);
            }
        }

        let input_state = match event.state {
            ElementState::Pressed => InputState::Pressed,
            ElementState::Released => InputState::Released,
        };

        let input_event = InputEvent::KeyboardEvent {
            key,
            state: input_state,
        };

        self.log_event(input_event);
    }

    pub fn mouse_input(&mut self, mouse_button: &MouseButton, state: &ElementState) {
        let down_state = match state {
            ElementState::Pressed => true,
            ElementState::Released => false,
        };

        match mouse_button {
            MouseButton::Left => self.left_mouse_down = down_state,
            MouseButton::Right => {
                self.right_mouse_down = down_state;
            }
            _ => {}
        };

        let input_state = match state {
            ElementState::Pressed => InputState::Pressed,
            ElementState::Released => InputState::Released,
        };

        let mouse_button_event = InputEvent::MouseButtonEvent {
            button: mouse_button.to_owned(),
            state: input_state,
            position: self.mouse_position,
        };

        self.log_event(mouse_button_event);
    }

    pub fn mouse_motion(&mut self, delta: (f32, f32)) {
        self.mouse_delta += Vec3::new(delta.0, delta.1, 0.0);
    }

    pub fn mouse_moved(&mut self, mouse_position: &PhysicalPosition<f64>) {
        self.mouse_position = Vec2::new(mouse_position.x as f32, mouse_position.y as f32)
    }

    pub fn mouse_wheel(&mut self, delta: MouseScrollDelta) {
        let delta = match delta {
            MouseScrollDelta::LineDelta(_x, y) => y,
            MouseScrollDelta::PixelDelta(physical_position) => physical_position.y as f32,
        };

        self.log_event(InputEvent::MouseScrollEvent { delta });
    }

    pub fn left_mouse_state(&self) -> Option<MouseButtonEvent> {
        self.input_event_record.left_mouse
    }

    pub fn right_mouse_state(&self) -> Option<MouseButtonEvent> {
        self.input_event_record.right_mouse
    }

    pub fn middle_mouse_state(&self) -> Option<MouseButtonEvent> {
        self.input_event_record.middle_mouse
    }

    pub fn mouse_button_down(&self, mouse_button: MouseButton) -> bool {
        match mouse_button {
            MouseButton::Left => self.left_mouse_down,
            MouseButton::Right => self.right_mouse_down,
            _ => {
                panic!("Unhandled button")
            }
        }
    }

    pub fn mouse_button_pressed(&self, mouse_button: MouseButton) -> bool {
        let to_check = match mouse_button {
            MouseButton::Left => self.input_event_record.left_mouse,
            MouseButton::Right => self.input_event_record.right_mouse,
            MouseButton::Middle => self.input_event_record.middle_mouse,
            _ => {
                panic!("Unhandled mouse button");
            }
        };

        let Some(event) = to_check else {
            return false;
        };

        event.state == InputState::Pressed
    }

    pub fn mouse_button_released(&self, mouse_button: MouseButton) -> bool {
        let to_check = match mouse_button {
            MouseButton::Left => self.input_event_record.left_mouse,
            MouseButton::Right => self.input_event_record.right_mouse,
            MouseButton::Middle => self.input_event_record.middle_mouse,
            _ => {
                return false;
            }
        };

        let Some(event) = to_check else {
            return false;
        };

        event.state == InputState::Released
    }

    pub fn key_pressed(&self, key: KeyCode) -> bool {
        self.input_event_record
            .key_states
            .get(&key)
            .map_or(false, |input_state| *input_state == InputState::Pressed)
    }

    pub fn key_release(&self, key: KeyCode) -> bool {
        self.input_event_record
            .key_states
            .get(&key)
            .map_or(false, |input_state| *input_state == InputState::Released)
    }

    pub fn is_key_down(&self, code: KeyCode) -> bool {
        self.keys_down.contains(&code)
    }

    pub fn clear_events(&mut self) {
        self.input_event_log.clear();
        self.input_event_record.clear();
    }

    pub fn get_input_events(&self) -> &[InputEvent] {
        &self.input_event_log
    }

    pub fn take_input_events(&mut self) -> Vec<InputEvent> {
        std::mem::take(&mut self.input_event_log)
    }

    pub fn simulate_event(&mut self, event: InputEvent) {
        self.log_event(event);
    }
}

#[cfg(test)]
mod tests {
    use super::{InputEvent::KeyboardEvent, InputState};
    use crate::input::InputHandler;
    use winit::keyboard::KeyCode;

    #[test]
    fn should_handle_key_events() {
        let mut input_handler = InputHandler::new();

        input_handler.simulate_event(KeyboardEvent {
            key: KeyCode::KeyW,
            state: InputState::Pressed,
        });

        assert_eq!(input_handler.key_pressed(KeyCode::KeyW), true);

        input_handler.simulate_event(KeyboardEvent {
            key: KeyCode::KeyW,
            state: InputState::Released,
        });

        assert_eq!(input_handler.key_pressed(KeyCode::KeyW), false)
    }
}
