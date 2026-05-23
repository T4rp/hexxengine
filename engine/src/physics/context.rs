use rapier3d::prelude::*;

pub struct PhysicsContext {
    pub gravity: Vec3,
    pub rigid_body_set: RigidBodySet,
    pub collider_set: ColliderSet,
    pub impulse_joint_set: ImpulseJointSet,
    pub multibody_joint_set: MultibodyJointSet,
    pub integration_parameters: IntegrationParameters,
    pub island_manager: IslandManager,
    pub broad_phase: DefaultBroadPhase,
    pub narrow_phase: NarrowPhase,
    pub ccd_solver: CCDSolver,
    pub physics_pipeline: PhysicsPipeline,
}

impl Default for PhysicsContext {
    fn default() -> Self {
        Self::new()
    }
}

impl PhysicsContext {
    pub fn new() -> Self {
        let rigid_body_set = RigidBodySet::new();
        let collider_set = ColliderSet::new();
        let impulse_joint_set = ImpulseJointSet::new();
        let multibody_joint_set = MultibodyJointSet::new();

        let gravity = Vec3::new(0.0, -196.0, 0.0);
        let integration_parameters = IntegrationParameters {
            length_unit: 1.0,
            ..Default::default()
        };
        let physics_pipeline = PhysicsPipeline::new();
        let island_manager = IslandManager::new();
        let broad_phase = DefaultBroadPhase::new();
        let narrow_phase = NarrowPhase::new();
        let ccd_solver = CCDSolver::new();

        Self {
            gravity,
            rigid_body_set,
            collider_set,
            impulse_joint_set,
            multibody_joint_set,
            broad_phase,
            integration_parameters,
            island_manager,
            ccd_solver,
            physics_pipeline,
            narrow_phase,
        }
    }

    pub fn step(&mut self) {
        self.physics_pipeline.step(
            self.gravity,
            &self.integration_parameters,
            &mut self.island_manager,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.rigid_body_set,
            &mut self.collider_set,
            &mut self.impulse_joint_set,
            &mut self.multibody_joint_set,
            &mut self.ccd_solver,
            &(),
            &(),
        );
    }
}
