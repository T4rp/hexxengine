use glam::{Quat, Vec3};

#[derive(Clone, Copy)]
pub struct TransformComponent {
    pub position: Vec3,
    pub orientation: Quat,
    pub size: Vec3,
}
