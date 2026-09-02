use glam::Vec3;
use rapier3d::geometry::ColliderHandle;

pub struct HandleInputState {
    pub is_holding: bool,
    pub is_hovering: bool,
    pub move_delta: Vec3,
}

pub struct HandleComponent {
    pub collider_handle: ColliderHandle,
    pub input_state: HandleInputState,
}
