use glam::Vec3;

use crate::components::TransformComponent;

pub struct SelectionBox {
    pub transform: TransformComponent,
    pub color: Vec3,
    pub transparency: f32,
}
