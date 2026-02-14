use std::collections::HashMap;

use glam::Vec3;
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, KeyEvent, MouseButton},
    keyboard::{KeyCode, PhysicalKey},
};

#[derive(Default, Debug)]
pub struct KeyboardInput {
    keys_down: HashMap<KeyCode, bool>,
    pub right_mouse_down: bool,
}

impl KeyboardInput {
    pub fn input(&mut self, event: KeyEvent) {
        if let PhysicalKey::Code(key) = event.physical_key {
            self.keys_down
                .entry(key)
                .insert_entry(event.state.is_pressed());
        };
    }

    pub fn is_key_down(&self, code: KeyCode) -> bool {
        *self.keys_down.get(&code).unwrap_or(&false)
    }
}

pub struct InputState {
    keys_down: HashMap<KeyCode, bool>,
    pub right_mouse_down: bool,
    pub left_mouse_down: bool,
    pub last_mouse_position: Vec3,
    pub mouse_delta: Vec3,
    pub right_clicked_on: Vec3,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            keys_down: HashMap::new(),
            right_mouse_down: false,
            left_mouse_down: false,
            last_mouse_position: Vec3::Z,
            mouse_delta: Vec3::ZERO,
            right_clicked_on: Vec3::ZERO,
        }
    }

    pub fn clear(&mut self) {
        self.mouse_delta = Vec3::ZERO
    }

    pub fn key_input(&mut self, event: &KeyEvent) {
        if let PhysicalKey::Code(key) = event.physical_key {
            self.keys_down
                .entry(key)
                .insert_entry(event.state.is_pressed());
        };
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
        }
    }

    pub fn mouse_motion(&mut self, delta: (f32, f32)) {
        self.mouse_delta += Vec3::new(delta.0, delta.1, 0.0);
    }

    pub fn mouse_moved(&mut self, mouse_position: &PhysicalPosition<f64>) {
        let mouse_position = Vec3::new(mouse_position.x as f32, mouse_position.y as f32, 0.0);
        self.last_mouse_position = mouse_position;
    }

    pub fn is_key_down(&self, code: KeyCode) -> bool {
        *self.keys_down.get(&code).unwrap_or(&false)
    }
}
