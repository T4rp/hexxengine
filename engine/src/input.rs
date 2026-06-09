use std::collections::{HashSet, VecDeque};

use glam::{Vec2, Vec3, Vec3Swizzles};
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, KeyEvent, MouseButton, RawKeyEvent},
    keyboard::{KeyCode, PhysicalKey},
};

pub enum InputState {
    Pressed,
    Released,
    Changed,
}

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

pub struct InputHandler {
    keys_down: HashSet<KeyCode>,
    pub right_mouse_down: bool,
    pub left_mouse_down: bool,

    pub mouse_position: Vec2,
    pub mouse_delta: Vec3,
    input_events: Vec<InputEvent>,

    #[deprecated = "Use input events instead"]
    pub right_clicked_on: Vec3,

    #[deprecated = "Use mouse_position instead"]
    pub last_mouse_position: Vec3,

    #[deprecated = "Use input events instead"]
    keys_pressed: HashSet<KeyCode>,
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
            input_events: Vec::new(),
            last_mouse_position: Vec3::Z,
            right_clicked_on: Vec3::ZERO,
            keys_pressed: HashSet::new(),
        }
    }

    pub fn clear(&mut self) {
        self.mouse_delta = Vec3::ZERO;
        self.keys_pressed.clear();
    }

    pub fn raw_key_input(&mut self, event: &RawKeyEvent) {
        let PhysicalKey::Code(key) = event.physical_key else {
            return;
        };

        match event.state {
            ElementState::Pressed => {
                if !self.keys_down.contains(&key) {
                    self.keys_pressed.insert(key);
                }
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
                if !self.keys_down.contains(&key) {
                    self.keys_pressed.insert(key);
                }
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

        self.input_events.push(input_event);
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
                self.right_clicked_on = self.last_mouse_position;
            }
            _ => {}
        };

        let input_state = match state {
            ElementState::Pressed => InputState::Pressed,
            ElementState::Released => InputState::Released,
        };

        let moues_button_event = InputEvent::MouseButtonEvent {
            button: mouse_button.to_owned(),
            state: input_state,
            position: self.last_mouse_position.xy(),
        };

        self.input_events.push(moues_button_event);
    }

    pub fn mouse_motion(&mut self, delta: (f32, f32)) {
        self.mouse_delta += Vec3::new(delta.0, delta.1, 0.0);
    }

    pub fn mouse_moved(&mut self, mouse_position: &PhysicalPosition<f64>) {
        self.last_mouse_position = Vec3::new(mouse_position.x as f32, mouse_position.y as f32, 0.0);
        self.mouse_position = Vec2::new(mouse_position.x as f32, mouse_position.y as f32)
    }

    pub fn is_key_down(&self, code: KeyCode) -> bool {
        self.keys_down.contains(&code)
    }

    #[deprecated = "Use input events instead"]
    pub fn is_key_pressed(&self, code: KeyCode) -> bool {
        self.keys_pressed.contains(&code)
    }

    pub fn clear_events(&mut self) {
        self.input_events.clear();
    }

    pub fn get_input_events(&self) -> &[InputEvent] {
        &self.input_events
    }
}
