use glam::{Quat, Vec3};
use rapier3d::{
    geometry::{ColliderBuilder, ColliderHandle, Group, InteractionGroups, InteractionTestMode},
    math::Pose,
};
use thunderdome::Index;

use crate::{
    components::TransformComponent,
    game::{CONE_MESH_ID, SPHERE_MESH_ID},
    physics::{HANDLE_INTERACTION_GROUP, context::PhysicsContext},
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
    pub collision_boxes: Vec<ColliderHandle>,
}

impl TransformHandles {
    pub fn new(
        selected: Index,
        physics: &mut PhysicsContext,
        transform_type: TransformType,
    ) -> Self {
        let mut collision_boxes = Vec::new();

        for _ in 0..HANDLE_AXES.len() {
            let collider = ColliderBuilder::ball(HANDLE_SIZE / 2.0)
                .collision_groups(HANDLE_INTERACTION_GROUP)
                .solver_groups(HANDLE_INTERACTION_GROUP)
                .build();

            let handle = physics.collider_set.insert(collider);

            collision_boxes.push(handle)
        }

        Self {
            selected,
            transform_type,
            collision_boxes,
        }
    }

    pub fn update(&mut self, physics: &mut PhysicsContext, transform: &TransformComponent) {
        for (i, axis) in HANDLE_AXES.iter().enumerate() {
            let direction = transform.orientation * axis;

            let collider_position = transform.position
                + direction * ((transform.size * axis).length() / 2.0 + HANDLE_OFFSET);

            let collider = physics
                .collider_set
                .get_mut(self.collision_boxes[i])
                .unwrap();

            collider.set_position(Pose::from_translation(collider_position));
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

            let collider_position = transform.position
                + direction * ((transform.size * axis).length() / 2.0 + HANDLE_OFFSET);

            let collider_orientation = Quat::from_rotation_arc(Vec3::Y, direction);

            render_scene.meshes.push(MeshNode {
                position: collider_position,
                orientation: collider_orientation,
                size: Vec3::splat(HANDLE_SIZE),
                color: HANDLE_AXES_COLOR[i],
                opacity: HANDLE_OPACITY,
                mesh_id: mesh_id,
                is_gizmo: true,
                ..Default::default()
            });
        }
    }
}
