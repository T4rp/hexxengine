use glam::{Quat, Vec3};
use rapier3d::math::{Pose, Pose3};

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

    pub fn as_pose(&self) -> Pose {
        Pose::from_parts(self.position, self.orientation)
    }
}
