use hexxengine::{
    components::{MeshComponent, RigidBodyComponent, TransformComponent},
    physics::context::PhysicsContext,
    thunderdome::{Arena, Index},
};

use crate::components::CharacterControllerComponent;

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

pub struct Character {
    pub transform: TransformComponent,
    pub mesh: MeshComponent,
    pub controller: CharacterControllerComponent,
}

pub struct World {
    pub parts: Arena<Part>,
    pub characters: Arena<Character>,
    pub character_index: Option<Index>,
}

impl World {
    pub fn new() -> World {
        Self {
            parts: Arena::new(),
            characters: Arena::new(),
            character_index: None,
        }
    }
}
