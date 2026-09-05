use glam::{Quat, Vec3};

#[derive(Clone, Copy)]
pub struct TransformComponent {
    pub position: Vec3,
    pub orientation: Quat,
    pub size: Vec3,
}

impl TransformComponent {
    pub fn new(position: Vec3, orientation: Quat, size: Vec3) -> Self {
        Self {
            position,
            orientation,
            size,
        }
    }
}
