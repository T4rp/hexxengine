use rapier3d::prelude::{RigidBodyType, ShapeType};

use crate::{
    components::{MeshComponent, RigidBodyComponent, TransformComponent},
    physics::context::PhysicsContext,
    renderer::render_scene::{MeshNode, RenderScene},
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
