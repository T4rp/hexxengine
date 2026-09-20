use glam::{Quat, Vec3};
use rapier3d::{
    geometry::{ColliderBuilder, ColliderHandle, Group, InteractionGroups, InteractionTestMode},
    math::Pose,
};
use thunderdome::Index;

use crate::{
    components::TransformComponent,
    game::{CONE_MESH_ID, SPHERE_MESH_ID},
    physics::{HANDLE_INTERACTION_GROUP, context::PhysicsContext, gen_userdata},
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

struct TransformHandle {
    transform: TransformComponent,
    collision_box: ColliderHandle,
    axis: Vec3,
    color: Vec3,
    mouse_hovering: bool,
    mouse_dragging: bool,
}

pub struct TransformHandles {
    pub selected: Index,
    pub transform_type: TransformType,
    pub handles: Vec<TransformHandle>,
    initialized: bool,
}

impl TransformHandles {
    pub fn new(
        selected: Index,
        physics: &mut PhysicsContext,
        transform_type: TransformType,
    ) -> Self {
        let mut handles = Vec::new();

        for i in 0..HANDLE_AXES.len() {
            let collider = ColliderBuilder::ball(HANDLE_SIZE / 2.0)
                .collision_groups(HANDLE_INTERACTION_GROUP)
                .solver_groups(HANDLE_INTERACTION_GROUP)
                .build();

            let collider_handle = physics.collider_set.insert(collider);

            let transform_handle = TransformHandle {
                transform: TransformComponent::new(
                    Vec3::ZERO,
                    Quat::IDENTITY,
                    Vec3::splat(HANDLE_SIZE),
                ),
                collision_box: collider_handle,
                axis: HANDLE_AXES[i],
                color: HANDLE_AXES_COLOR[i],
                mouse_hovering: false,
                mouse_dragging: false,
            };

            handles.push(transform_handle)
        }

        Self {
            selected,
            transform_type,
            handles,
            initialized: false,
        }
    }

    pub fn update(&mut self, physics: &mut PhysicsContext, transform: &TransformComponent) {
        assert_eq!(self.initialized, true, "handle must be initialized");

        for handle in self.handles.iter_mut() {
            let direction = transform.orientation * handle.axis;

            let handle_position = transform.position
                + direction * ((transform.size * handle.axis).length() / 2.0 + HANDLE_OFFSET);

            let handle_orientation = Quat::from_rotation_arc(Vec3::Y, direction);

            handle.transform.position = handle_position;
            handle.transform.orientation = handle_orientation;

            let collider = physics.collider_set.get_mut(handle.collision_box).unwrap();
            collider.set_position(handle.transform.as_pose());
        }
    }

    pub fn init_colliders(&mut self, physics: &mut PhysicsContext, index: u64) {
        for (i, handle) in self.handles.iter_mut().enumerate() {
            let collider = physics.collider_set.get_mut(handle.collision_box).unwrap();
            collider.user_data = gen_userdata(crate::physics::UserdataType::Handle, index, i as u32)
        }

        self.initialized = true
    }

    pub fn draw(&self, render_scene: &mut RenderScene, transform: &TransformComponent) {
        let mesh_id = match self.transform_type {
            TransformType::Position => CONE_MESH_ID,
            TransformType::Size => SPHERE_MESH_ID,
            TransformType::Rotation => unimplemented!(),
        };

        for handle in self.handles.iter() {
            render_scene.meshes.push(MeshNode {
                position: handle.transform.position,
                orientation: handle.transform.orientation,
                size: handle.transform.size,
                color: handle.color,
                opacity: HANDLE_OPACITY,
                mesh_id: mesh_id,
                is_gizmo: true,
                ..Default::default()
            });
        }
    }
}
