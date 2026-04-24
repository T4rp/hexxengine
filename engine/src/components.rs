use glam::{Quat, Vec3};
use rapier3d::{
    math::Pose3,
    prelude::{
        ColliderBuilder, ColliderHandle, RigidBodyBuilder, RigidBodyHandle, RigidBodyType,
        ShapeType, SharedShape,
    },
};
use thunderdome::Index;

use crate::physics::context::PhysicsContext;

#[derive(Clone, Copy)]
pub struct TransformComponent {
    pub position: Vec3,
    pub orientation: Quat,
    pub size: Vec3,
}

pub struct MeshComponent {
    pub color: Vec3,
    pub mesh_id: Index,
    pub material: Index,
    pub opacity: f32,
}

pub struct RigidBodyComponent {
    pub collider_handle: ColliderHandle,
    pub rigid_body_handle: RigidBodyHandle,
    pub shape_type: ShapeType,
}

impl RigidBodyComponent {
    pub fn new(
        phys_ctx: &mut PhysicsContext,
        transform: &TransformComponent,
        body_type: RigidBodyType,
        shape_type: ShapeType,
    ) -> RigidBodyComponent {
        let shape = match shape_type {
            ShapeType::Ball => {
                let radius = transform.size.length() / 2.0;
                SharedShape::ball(radius)
            }
            ShapeType::Cuboid => SharedShape::cuboid(
                transform.size.x / 2.0,
                transform.size.y / 2.0,
                transform.size.z / 2.0,
            ),
            _ => {
                panic!("unsupported shape: {:?}", shape_type)
            }
        };

        let collider = ColliderBuilder::new(shape).build();

        let rigid_body = RigidBodyBuilder::new(body_type)
            .pose(Pose3::from_parts(transform.position, transform.orientation))
            .build();

        let rigid_body_handle = phys_ctx.rigid_body_set.insert(rigid_body);

        let collider_handle = phys_ctx.collider_set.insert_with_parent(
            collider,
            rigid_body_handle,
            &mut phys_ctx.rigid_body_set,
        );

        Self {
            shape_type,
            collider_handle,
            rigid_body_handle,
        }
    }

    pub fn destroy(&self, phys_ctx: &mut PhysicsContext) {
        phys_ctx.rigid_body_set.remove(
            self.rigid_body_handle,
            &mut phys_ctx.island_manager,
            &mut phys_ctx.collider_set,
            &mut phys_ctx.impulse_joint_set,
            &mut phys_ctx.multibody_joint_set,
            true,
        );
    }
}
