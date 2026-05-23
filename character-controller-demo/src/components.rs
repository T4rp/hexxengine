use hexxengine::{
    components::TransformComponent,
    glam::{Quat, Vec3},
    physics::{
        character_controller::{CharacterCollision, CharacterLength, KinematicCharacterController},
        context::PhysicsContext,
    },
    rapier3d::{
        math::Pose3,
        prelude::{
            ColliderBuilder, ColliderHandle, MassProperties, QueryFilter, SharedShape,
        },
    },
    thunderdome::Index,
};

const JUMP_POWER: f32 = 70.0;
const GROUND_SPEED: f32 = 47.0;
const AIR_SPEED: f32 = 6.0;
const STOP_SPEED: f32 = 19.0;
const GROUND_ACCEL: f32 = 10.0;
const AIR_ACCEL: f32 = 100.0;
const FRICTION: f32 = 6.0;

type EntityIndex<T> = (T, Index);

pub struct HierarchyComponent<T> {
    parent: Option<EntityIndex<T>>,
    children: Vec<EntityIndex<T>>,
}

pub struct CharacterControllerComponent {
    pub collider: ColliderHandle,
    pub mass_properties: MassProperties,
    pub character_controller: KinematicCharacterController,
    pub move_dir: Vec3,
    pub position: Vec3,
    pub velocity: Vec3,
    pub jump: bool,
    pub grounded: bool,
    pub collisions: Vec<CharacterCollision>,
    pub shape: SharedShape,
}

impl CharacterControllerComponent {
    pub fn new(
        phys_ctx: &mut PhysicsContext,
        transform: &TransformComponent,
    ) -> CharacterControllerComponent {
        let mut character_controller = KinematicCharacterController::default();
        character_controller.max_slope_climb_angle = 45.5_f32.to_radians();
        character_controller.min_slope_slide_angle = 45.5_f32.to_radians();
        character_controller.offset = CharacterLength::Absolute(0.2);
        character_controller.normal_nudge_factor = 1.0e-3;

        let capsule_shape = SharedShape::capsule_y(
            transform.size.y / 2.0,
            (transform.size.x + transform.size.z) / 2.0 / 2.0,
        );
        let mass_properties = capsule_shape.mass_properties(1.0);

        let collider = ColliderBuilder::new(capsule_shape.clone()).build();
        let collider_handle = phys_ctx.collider_set.insert(collider);

        Self {
            shape: capsule_shape,
            position: transform.position,
            velocity: Vec3::ZERO,
            collider: collider_handle,
            character_controller,
            move_dir: Vec3::ZERO,
            jump: false,
            grounded: false,
            collisions: Vec::new(),
            mass_properties,
        }
    }

    fn accel(&mut self, dt: f32, wish_dir: Vec3, wish_speed: f32, accel: f32) {
        let mut vel = self.velocity;
        vel.y = 0.0;

        let current_speed = vel.dot(wish_dir);
        let mut add_speed = wish_speed - current_speed;

        if add_speed <= 0.0 {
            add_speed = 0.0
        }

        let mut accel_speed = accel * wish_speed * dt;

        if accel_speed > add_speed {
            accel_speed = add_speed;
        }

        self.velocity += wish_dir * accel_speed;
    }

    fn friction(&mut self, dt: f32, friction: f32) {
        let mut vel = self.velocity;

        if self.grounded {
            vel.y = 0.0;
        }

        let speed = vel.length();

        if speed < 0.1 {
            return;
        }

        let control = speed.max(STOP_SPEED);
        let drop = control * friction * dt;

        let mut new_speed = speed - drop;
        if new_speed < 0.0 {
            new_speed = 0.0;
        }

        new_speed /= speed;

        self.velocity.x *= new_speed;
        self.velocity.z *= new_speed;
    }

    fn solve_colisions(&mut self, phys_ctx: &mut PhysicsContext, dt: f32) {
        let filter = QueryFilter::new().exclude_collider(self.collider);

        let mut query_pipeline = phys_ctx.broad_phase.as_query_pipeline_mut(
            phys_ctx.narrow_phase.query_dispatcher(),
            &mut phys_ctx.rigid_body_set,
            &mut phys_ctx.collider_set,
            filter,
        );

        self.character_controller
            .solve_character_collision_impulses(
                dt,
                &mut query_pipeline,
                self.shape.clone_dyn().as_ref(),
                self.mass_properties.mass(),
                &self.collisions,
            );
    }

    pub fn move_dir(&mut self, phys_ctx: &mut PhysicsContext, dt: f32) {
        self.collisions.clear();

        if !self.grounded {
            self.velocity += phys_ctx.gravity * dt;
        }

        if self.grounded {
            self.velocity.y = 0.0;
            if self.jump {
                self.velocity.y = JUMP_POWER;
                self.grounded = false;
            }
        }

        let wish_dir = self.move_dir.normalize_or_zero();

        if self.grounded {
            self.friction(dt, FRICTION);
            self.accel(dt, wish_dir, GROUND_SPEED, GROUND_ACCEL);
        } else {
            self.accel(dt, wish_dir, AIR_SPEED, AIR_ACCEL);
        }

        let filter = QueryFilter::new().exclude_collider(self.collider);

        let query_pipeline = phys_ctx.broad_phase.as_query_pipeline(
            phys_ctx.narrow_phase.query_dispatcher(),
            &phys_ctx.rigid_body_set,
            &phys_ctx.collider_set,
            filter,
        );

        let movement = self.character_controller.move_shape(
            dt,
            &query_pipeline,
            self.shape.clone_dyn().as_ref(),
            &Pose3 {
                rotation: Quat::IDENTITY,
                translation: self.position,
            },
            self.velocity * dt,
            |collision| self.collisions.push(collision),
        );

        self.grounded = movement.grounded;
        self.velocity = movement.translation / dt;
        self.position += movement.translation;
        self.jump = false;

        self.solve_colisions(phys_ctx, dt);
    }

    pub fn update_position(&self, dt: f32, transform: &mut TransformComponent) {
        transform.position = self.position + self.velocity * dt;
    }
}
