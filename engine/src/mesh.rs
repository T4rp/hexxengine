use glam::Vec2;

use crate::renderer::scene3d::MeshVertex;

pub struct MeshData {
    pub vertices: Vec<MeshVertex>,
    pub uvs: Vec<Vec2>,
    pub indices: Vec<u16>,
}
