use glam::Vec3;
use thunderdome::Index;

pub struct MeshComponent {
    pub color: Vec3,
    pub mesh_id: Index,
    pub material: Index,
    pub opacity: f32,
}
