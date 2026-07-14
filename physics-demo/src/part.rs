use hexxengine::{
    glam::{Quat, Vec3},
    physics::context::PhysicsContext,
    rapier3d::{
        dynamics::{RigidBodyBuilder, RigidBodyHandle, RigidBodyType},
        geometry::{ColliderBuilder, ColliderHandle},
        math::Pose3,
    },
};

pub enum PartShape {
    Cube(Vec3),
    Sphere(f32),
}

pub struct Part {
    pub position: Vec3,
    pub orientation: Quat,
    pub shape: PartShape,
    pub color: Vec3,
    pub collider: ColliderHandle,
    pub rigid_body_handle: RigidBodyHandle,
}

impl Part {
    pub fn new_cube(
        phys_ctx: &mut PhysicsContext,
        body_type: RigidBodyType,
        position: Vec3,
        orientation: Quat,
        size: Vec3,
        color: Vec3,
    ) -> Self {
        let collider = ColliderBuilder::cuboid(size.x / 2.0, size.y / 2.0, size.z / 2.0).build();

        let rigid_body = RigidBodyBuilder::new(body_type)
            .pose(Pose3::from_parts(position, orientation))
            .build();

        let rigid_body_handle = phys_ctx.rigid_body_set.insert(rigid_body);

        let collider_handle = phys_ctx.collider_set.insert_with_parent(
            collider,
            rigid_body_handle,
            &mut phys_ctx.rigid_body_set,
        );

        Self {
            position,
            orientation,
            shape: PartShape::Cube(size),
            color,
            collider: collider_handle,
            rigid_body_handle,
        }
    }

    pub fn new_sphere(
        phys_ctx: &mut PhysicsContext,
        body_type: RigidBodyType,
        position: Vec3,
        orientation: Quat,
        radius: f32,
        color: Vec3,
    ) -> Self {
        let collider = ColliderBuilder::ball(radius).build();

        let rigid_body = RigidBodyBuilder::new(body_type)
            .pose(Pose3::from_parts(position, orientation))
            .build();

        let rigid_body_handle = phys_ctx.rigid_body_set.insert(rigid_body);

        let collider_handle = phys_ctx.collider_set.insert_with_parent(
            collider,
            rigid_body_handle,
            &mut phys_ctx.rigid_body_set,
        );

        Self {
            position,
            orientation,
            shape: PartShape::Sphere(radius),
            color,
            collider: collider_handle,
            rigid_body_handle,
        }
    }

    pub fn destroy(self, phys_ctx: &mut PhysicsContext) {
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
