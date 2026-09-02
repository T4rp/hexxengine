use rapier3d::prelude::{RigidBodyType, ShapeType};

use crate::{
    components::{MeshComponent, RigidBodyComponent, TransformComponent},
    physics::context::PhysicsContext,
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

    pub fn destroy(self, phys_ctx: &mut PhysicsContext) {
        self.rigid_body.destroy(phys_ctx);
    }
}
