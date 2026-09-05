use glam::{Quat, Vec3};
use rapier3d::prelude::{RigidBodyType, ShapeType};

use crate::{
    components::{MeshComponent, RigidBodyComponent, TransformComponent},
    game::{CONE_MESH_ID, CUBE_MESH_ID, SPHERE_MESH_ID},
    physics::context::PhysicsContext,
    renderer::{
        render_scene::{MeshNode, RenderScene},
        renderer::BASE_MATERIAL_INDEX,
    },
};

pub struct Part {
    pub transform: TransformComponent,
    pub mesh: MeshComponent,
    pub rigid_body: RigidBodyComponent,
}

impl Part {
    pub fn new(
        phys_ctx: &mut PhysicsContext,
        transform: TransformComponent,
        mesh: MeshComponent,
        rigid_body_type: RigidBodyType,
        shape_type: ShapeType,
    ) -> Self {
        Self {
            transform,
            mesh,
            rigid_body: RigidBodyComponent::new(phys_ctx, &transform, rigid_body_type, shape_type),
        }
    }

    pub fn new_cube(
        phys_ctx: &mut PhysicsContext,
        body_type: RigidBodyType,
        position: Vec3,
        orientation: Quat,
        size: Vec3,
        color: Vec3,
    ) -> Self {
        Self::new(
            phys_ctx,
            TransformComponent::new(position, orientation, size),
            MeshComponent {
                color,
                mesh_id: CUBE_MESH_ID,
                material: BASE_MATERIAL_INDEX,
                opacity: 1.0,
            },
            body_type,
            ShapeType::Cuboid,
        )
    }

    pub fn new_sphere(
        phys_ctx: &mut PhysicsContext,
        body_type: RigidBodyType,
        position: Vec3,
        orientation: Quat,
        radius: f32,
        color: Vec3,
    ) -> Self {
        Self::new(
            phys_ctx,
            TransformComponent::new(position, orientation, Vec3::splat(radius)),
            MeshComponent {
                color,
                mesh_id: SPHERE_MESH_ID,
                material: BASE_MATERIAL_INDEX,
                opacity: 1.0,
            },
            body_type,
            ShapeType::Ball,
        )
    }

    pub fn new_cone(
        phys_ctx: &mut PhysicsContext,
        body_type: RigidBodyType,
        position: Vec3,
        orientation: Quat,
        radius: f32,
        height: f32,
        color: Vec3,
    ) -> Self {
        Self::new(
            phys_ctx,
            TransformComponent::new(position, orientation, Vec3::new(radius, height, radius)),
            MeshComponent {
                color,
                mesh_id: CONE_MESH_ID,
                material: BASE_MATERIAL_INDEX,
                opacity: 1.0,
            },
            body_type,
            ShapeType::Cone,
        )
    }

    pub fn update(&mut self, phyx_ctx: &mut PhysicsContext, dt: f32) {
        let rigid_body = phyx_ctx
            .rigid_body_set
            .get(self.rigid_body.rigid_body_handle)
            .unwrap();

        let pose = rigid_body.position();

        let pos_interpolated = rigid_body.predict_position_using_velocity(dt);

        self.transform.position = pos_interpolated.translation;
        self.transform.orientation = pos_interpolated.rotation;
    }

    pub fn draw(&self, scene: &mut RenderScene) {
        scene.meshes.push(MeshNode {
            position: self.transform.position,
            orientation: self.transform.orientation,
            size: self.transform.size,
            color: self.mesh.color,
            opacity: self.mesh.opacity,
            mesh_id: self.mesh.mesh_id,
            material_id: self.mesh.material,
        });
    }

    pub fn destroy(self, phys_ctx: &mut PhysicsContext) {
        self.rigid_body.destroy(phys_ctx);
    }
}
