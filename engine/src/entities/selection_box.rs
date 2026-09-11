use glam::Vec3;
use thunderdome::Index;

use crate::{
    components::TransformComponent,
    game::CUBE_MESH_ID,
    renderer::{
        render_scene::{MeshNode, RenderScene},
        renderer::BASE_MATERIAL_INDEX,
    },
};

pub struct SelectionBox {
    pub selected: Index,
    pub color: Vec3,
    pub opacity: f32,
}

impl SelectionBox {
    pub fn new(index: Index) -> Self {
        SelectionBox {
            selected: index,
            color: Vec3::new(0.0, 0.0, 1.0),
            opacity: 0.4,
        }
    }

    pub fn draw(&self, scene: &mut RenderScene, transform: &TransformComponent) {
        scene.meshes.push(MeshNode {
            position: transform.position,
            orientation: transform.orientation,
            size: transform.size + Vec3::splat(0.5),
            color: self.color,
            opacity: self.opacity,
            mesh_id: CUBE_MESH_ID,
            material_id: BASE_MATERIAL_INDEX,
        });
    }
}
