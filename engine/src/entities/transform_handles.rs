use glam::{Quat, Vec3};
use thunderdome::Index;

use crate::{
    components::TransformComponent,
    game::{CONE_MESH_ID, SPHERE_MESH_ID},
    renderer::render_scene::{MeshNode, RenderScene},
};

const HANDLE_OFFSET: f32 = 5.0;
const HANDLE_SIZE: f32 = 5.0;
const HANDLE_OPACITY: f32 = 0.7;

const HANDLE_AXES: &'static [Vec3] = &[
    Vec3::X,
    Vec3::Y,
    Vec3::NEG_Z,
    Vec3::NEG_X,
    Vec3::NEG_Y,
    Vec3::Z,
];

const HANDLE_AXES_COLOR: &'static [Vec3] = &[Vec3::X, Vec3::Y, Vec3::Z, Vec3::X, Vec3::Y, Vec3::Z];

#[derive(Debug, PartialEq, Eq)]
pub enum TransformType {
    Position,
    Rotation,
    Size,
}

pub struct TransformHandles {
    pub selected: Index,
    pub transform_type: TransformType,
}

impl TransformHandles {
    pub fn new(selected: Index, transform_type: TransformType) -> Self {
        Self {
            selected,
            transform_type,
        }
    }

    pub fn draw(&self, render_scene: &mut RenderScene, transform: &TransformComponent) {
        let mesh_id = match self.transform_type {
            TransformType::Position => CONE_MESH_ID,
            TransformType::Size => SPHERE_MESH_ID,
            TransformType::Rotation => unimplemented!(),
        };

        for (i, axis) in HANDLE_AXES.iter().enumerate() {
            let direction = transform.orientation * axis;

            render_scene.meshes.push(MeshNode {
                position: transform.position
                    + direction * ((transform.size * axis).length() / 2.0 + HANDLE_OFFSET),
                orientation: Quat::from_rotation_arc(Vec3::Y, direction),
                size: Vec3::splat(HANDLE_SIZE),
                color: HANDLE_AXES_COLOR[i],
                opacity: HANDLE_OPACITY,
                mesh_id: CONE_MESH_ID,
                is_gizmo: true,
                ..Default::default()
            });
        }
    }
}
