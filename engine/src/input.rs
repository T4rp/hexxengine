use std::collections::HashSet;

use glam::Vec3;
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, KeyEvent, MouseButton, RawKeyEvent},
    keyboard::{KeyCode, PhysicalKey},
};

pub struct InputState {
    keys_down: HashSet<KeyCode>,
    keys_pressed: HashSet<KeyCode>,
    pub right_mouse_down: bool,
    pub left_mouse_down: bool,
    pub last_mouse_position: Vec3,
    pub mouse_delta: Vec3,
    pub right_clicked_on: Vec3,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            keys_down: HashSet::with_capacity(50),
            keys_pressed: HashSet::with_capacity(10),
            right_mouse_down: false,
            left_mouse_down: false,
            last_mouse_position: Vec3::Z,
            mouse_delta: Vec3::ZERO,
            right_clicked_on: Vec3::ZERO,
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
        self.keys_down.contains(&code)
    }

    pub fn is_key_pressed(&self, code: KeyCode) -> bool {
        self.keys_pressed.contains(&code)
    }
}
