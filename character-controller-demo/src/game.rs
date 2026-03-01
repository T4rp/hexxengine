use std::time::Instant;

use hexxengine::{
    assets::ASSET_PATH,
    glam, rand,
    rapier3d::{
        self,
        parry::shape::Capsule,
        prelude::{QueryFilter, QueryPipeline, Shape, ShapeType},
    },
    thunderdome, winit,
};

use glam::{EulerRot, Quat, Vec3, vec3};
use rapier3d::{
    control::KinematicCharacterController,
    math::Pose3,
    prelude::{ColliderBuilder, ColliderHandle, RigidBodyBuilder, RigidBodyHandle, RigidBodyType},
};

use rand::{Rng, SeedableRng, rngs::SmallRng};
use thunderdome::Arena;
use winit::{
    event::{DeviceEvent, WindowEvent},
    keyboard::KeyCode,
    window::Window,
};

use hexxengine::{
    assets::{get_first_gltf_mesh, load_skybox},
    color::hsv_to_rgb,
    input::InputState,
    physics::PhysicsContext,
    renderer::renderer::{MeshHandle, VulkanContext},
    scene::{Camera, Lighting, MeshNode, RenderScene},
};

const CAMERA_SPEED: f32 = 100.0;
const STEP_HZ: f32 = 1.0 / 60.0;
const CHARACTER_HEIGHT: f32 = 10.0;
const CHARACTER_RADIUS: f32 = 2.0;

enum PartShape {
    Cube(Vec3),
    Sphere(f32),
}

struct Part {
    position: Vec3,
    orientation: Quat,
    shape: PartShape,
    color: Vec3,
    collider: ColliderHandle,
    rigid_body_handle: RigidBodyHandle,
}

impl Part {
    fn new_cube(
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
            rigid_body_handle: rigid_body_handle,
        }
    }

    fn new_sphere(
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
            rigid_body_handle: rigid_body_handle,
        }
    }

    fn destroy(self, phys_ctx: &mut PhysicsContext) {
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

struct Character {
    position: Vec3,
    orientation: Quat,
    rigid_body: RigidBodyHandle,
    collider: ColliderHandle,
    character_controller: KinematicCharacterController,
    move_dir: Vec3,
    velocity: Vec3,
    jump: bool,
    grounded: bool,
}

impl Character {
    fn new(phys_ctx: &mut PhysicsContext, position: Vec3) -> Self {
        let character_controller = KinematicCharacterController::default();

        let collider = ColliderBuilder::capsule_y(CHARACTER_HEIGHT / 2.0, CHARACTER_RADIUS).build();

        let rigid_body = RigidBodyBuilder::new(RigidBodyType::KinematicVelocityBased)
            .pose(Pose3::from_parts(position, Quat::IDENTITY))
            .build();

        let rigid_body_handle = phys_ctx.rigid_body_set.insert(rigid_body);

        let collider_handle = phys_ctx.collider_set.insert_with_parent(
            collider,
            rigid_body_handle,
            &mut phys_ctx.rigid_body_set,
        );

        Self {
            position,
            orientation: Quat::IDENTITY,
            rigid_body: rigid_body_handle,
            velocity: Vec3::ZERO,
            collider: collider_handle,
            character_controller,
            move_dir: Vec3::ZERO,
            jump: false,
            grounded: false,
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

        let control = speed.max(30.0);
        let drop = control * friction * dt;

        let mut new_speed = speed - drop;
        if new_speed < 0.0 {
            new_speed = 0.0;
        }

        new_speed /= speed;

        self.velocity *= new_speed;
    }

    fn move_dir(&mut self, phys_ctx: &mut PhysicsContext, dt: f32) {
        if !self.grounded {
            self.velocity += phys_ctx.gravity * dt;
        }

        if self.grounded {
            self.velocity.y = 0.0;
            if self.jump {
                self.velocity.y = 75.0;
                self.grounded = false;
            }
        }

        let wish_dir = self.move_dir.normalize_or_zero();

        if self.grounded {
            self.friction(dt, 6.0);
            self.accel(dt, wish_dir, 48.0, 5.0);
        } else {
            self.accel(dt, wish_dir, 48.0, 10.0);
        }

        let mut xy = self.velocity * Vec3::new(1.0, 0.0, 1.0);
        xy = xy.clamp_length_max(150.0);
        self.velocity.x = xy.x;
        self.velocity.z = xy.z;

        let shape = phys_ctx.collider_set.get(self.collider).unwrap().shape();

        let filter = QueryFilter::new().exclude_rigid_body(self.rigid_body);

        let current_rigid_body_position = phys_ctx
            .rigid_body_set
            .get(self.rigid_body)
            .unwrap()
            .position();

        let query_pipeline = phys_ctx.broad_phase.as_query_pipeline(
            phys_ctx.narrow_phase.query_dispatcher(),
            &phys_ctx.rigid_body_set,
            &phys_ctx.collider_set,
            filter,
        );

        let movement = self.character_controller.move_shape(
            dt,
            &query_pipeline,
            shape,
            current_rigid_body_position,
            self.velocity * dt,
            |_| {},
        );

        self.grounded = movement.grounded;
        self.velocity = movement.translation / dt;

        self.jump = false;

        let rigid_body = phys_ctx.rigid_body_set.get_mut(self.rigid_body).unwrap();

        rigid_body.set_linvel(self.velocity, true);

        let body_pos = rigid_body.position();
        self.position = body_pos.translation;
        self.orientation = body_pos.rotation;
    }
}

struct GameResources {
    cube_mesh: MeshHandle,
    sphere_mesh: MeshHandle,
    skybox1: u32,
}

pub struct Game {
    vk_ctx: VulkanContext,
    input_state: InputState,
    scene: RenderScene,
    last_frame: Instant,
    start_time: Instant,
    rng: SmallRng,
    resources: GameResources,
    parts: Arena<Part>,
    physics_context: PhysicsContext,
    accumulator: f32,
    character: Character,
}

impl Game {
    pub fn new(window: &Window) -> Self {
        let mut vk_ctx = VulkanContext::new(&window);

        let cube_mesh = get_first_gltf_mesh(format!("{}/cube.gltf", ASSET_PATH).as_str());
        let sphere_mesh = get_first_gltf_mesh(format!("{}/sphere.gltf", ASSET_PATH).as_str());

        let cube_mesh = vk_ctx.load_mesh(&cube_mesh.vertices, &cube_mesh.indices);
        let sphere_mesh = vk_ctx.load_mesh(&sphere_mesh.vertices, &sphere_mesh.indices);

        let skybox1_id = load_skybox(
            &mut vk_ctx,
            format!(
                "{}/cloudy-skyboxes/Cubemap/Cubemap_Sky_04-512x512.png",
                ASSET_PATH
            )
            .as_str(),
        );

        let resources = GameResources {
            cube_mesh,
            sphere_mesh,
            skybox1: skybox1_id,
        };

        let start_time = Instant::now();
        let last_frame = start_time;

        let scene = RenderScene {
            camera: Camera::new(
                vec3(0.0, 100.0, 100.0),
                Quat::from_euler(EulerRot::ZXY, 0.0, f32::to_radians(-45.0), 0.0),
                120.0,
            ),
            meshes: Vec::new(),
            lighting: Lighting {
                sun_direction: vec3(0.0, -1.0, -1.0).normalize(),
                sun_color: vec3(1.0, 0.95, 0.85),
                sun_power: 0.5,
                ambient_color: vec3(0.9, 0.95, 1.0) * 0.2,
                skybox_id: skybox1_id,
            },
        };

        let mut rng = SmallRng::from_os_rng();
        let mut physics_context = PhysicsContext::new();
        let mut cubes = Arena::new();

        cubes.insert(Part::new_cube(
            &mut physics_context,
            RigidBodyType::Fixed,
            vec3(0.0, -25.0, 0.0),
            Quat::IDENTITY,
            vec3(2048.0, 50.0, 2048.0),
            vec3(0.8, 0.8, 0.8),
        ));

        let character = Character::new(&mut physics_context, vec3(0.0, 25.0, 0.0));

        let input_state = InputState::new();

        Self {
            vk_ctx,
            scene,
            last_frame,
            start_time,
            input_state,
            rng,
            resources,
            parts: cubes,
            physics_context,
            accumulator: 0.0,
            character,
        }
    }

    fn update_fixed(&mut self) {
        self.character.move_dir(&mut self.physics_context, STEP_HZ);
        self.physics_context.step();
    }

    fn update_character_movement(&mut self) {
        let mut move_dir = Vec3::ZERO;

        if self.input_state.is_key_down(KeyCode::KeyA) {
            move_dir += Vec3::new(-1.0, 0.0, 0.0)
        }

        if self.input_state.is_key_down(KeyCode::KeyD) {
            move_dir += Vec3::new(1.0, 0.0, 0.0)
        }

        if self.input_state.is_key_down(KeyCode::KeyW) {
            move_dir += Vec3::new(0.0, 0.0, -1.0)
        }

        if self.input_state.is_key_down(KeyCode::KeyS) {
            move_dir += Vec3::new(0.0, 0.0, 1.0)
        }

        if self.input_state.is_key_down(KeyCode::Space) {
            self.character.jump = true;
        }

        let mut world_move = self.scene.camera.orientation * move_dir;
        world_move.y = 0.0;

        world_move = world_move.normalize_or_zero();

        self.character.move_dir = world_move;
    }

    fn update_camera(&mut self, _dt: f32, window: &Window) {
        let camera = &mut self.scene.camera;

        if self.input_state.right_mouse_down {
            let _ = window
                .set_cursor_grab(winit::window::CursorGrabMode::Confined)
                .or_else(|_| window.set_cursor_grab(winit::window::CursorGrabMode::Locked));
            window.set_cursor_visible(false);
        } else {
            let _ = window.set_cursor_grab(winit::window::CursorGrabMode::None);
            window.set_cursor_visible(true);
        }

        let mouse_delta = self.input_state.mouse_delta;

        if mouse_delta.z == 0.0 && self.input_state.right_mouse_down {
            let sensitivity = 0.001;

            let yaw = Quat::from_rotation_y(-mouse_delta.x * sensitivity);
            let pitch = Quat::from_rotation_x(-mouse_delta.y * sensitivity);

            camera.orientation = yaw * camera.orientation * pitch;
        }

        camera.position = self.character.position + Vec3::new(0.0, CHARACTER_HEIGHT / 2.0, 0.0);
    }

    fn clean_parts(&mut self) {
        let mut to_remove = Vec::new();

        for (index, cube) in self.parts.iter_mut() {
            let rigid_body = self
                .physics_context
                .rigid_body_set
                .get(cube.rigid_body_handle)
                .unwrap();

            let pose = rigid_body.position();

            if pose.translation.y < -500.0 {
                to_remove.push(index);
            }

            cube.position = pose.translation;
            cube.orientation = pose.rotation;
        }

        for index in to_remove {
            let Some(cube) = self.parts.remove(index) else {
                continue;
            };

            cube.destroy(&mut self.physics_context);
        }
    }

    pub fn update(&mut self, window: &Window) {
        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32();
        let elapsed = (now - self.start_time).as_secs_f32();

        self.last_frame = now;
        self.accumulator += dt;

        self.update_character_movement();

        let gravity_y = self.physics_context.gravity.y;

        while self.accumulator > STEP_HZ {
            self.update_fixed();
            self.accumulator -= STEP_HZ;
        }

        self.clean_parts();
        self.update_camera(dt, window);

        self.input_state.clear();
    }

    fn draw(&mut self) {
        self.scene.meshes.clear();

        for (_i, part) in self.parts.iter() {
            match part.shape {
                PartShape::Cube(size) => {
                    self.scene.meshes.push(MeshNode {
                        position: part.position,
                        orientation: part.orientation,
                        size: size,
                        color: part.color,
                        opacity: 1.0,
                        mesh_id: self.resources.cube_mesh,
                        material_id: 0,
                    });
                }
                PartShape::Sphere(radius) => {
                    self.scene.meshes.push(MeshNode {
                        position: part.position,
                        orientation: part.orientation,
                        size: Vec3::splat(radius * 2.0),
                        color: part.color,
                        opacity: 1.0,
                        mesh_id: self.resources.sphere_mesh,
                        material_id: 0,
                    });
                }
            }
        }

        self.scene.meshes.push(MeshNode {
            position: self.character.position,
            orientation: self.character.orientation,
            size: vec3(
                CHARACTER_RADIUS * 2.0,
                CHARACTER_HEIGHT + CHARACTER_RADIUS * 2.0,
                CHARACTER_RADIUS * 2.0,
            ),
            color: vec3(0.0, 0.0, 0.0),
            opacity: 1.0,
            mesh_id: self.resources.cube_mesh,
            material_id: 1,
        });

        self.vk_ctx.draw(&self.scene);
    }

    pub fn handle_device_event(&mut self, event: &DeviceEvent) {
        match event {
            DeviceEvent::MouseMotion { delta } => {
                self.input_state
                    .mouse_motion((delta.0 as f32, delta.1 as f32));
            }
            DeviceEvent::Key(key_event) => {
                self.input_state.raw_key_input(key_event);
            }
            _ => {}
        }
    }

    pub fn handle_window_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => {
                // self.input_state.key_input(event);
            }
            WindowEvent::MouseInput {
                device_id: _,
                state,
                button,
            } => {
                self.input_state.mouse_input(button, state);
            }
            WindowEvent::CursorMoved {
                device_id: _,
                position,
            } => {
                self.input_state.mouse_moved(position);
            }
            WindowEvent::Resized(size) => {
                self.vk_ctx.handle_resize((size.width, size.height));
            }
            WindowEvent::RedrawRequested => self.draw(),
            _ => {}
        }
    }
}
