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
    pub fn destroy(self, phys_ctx: &mut PhysicsContext) {
        self.rigid_body.destroy(phys_ctx);
    }
}
