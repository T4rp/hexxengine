use std::time::Instant;

use hexxengine::{
    assets::ASSET_PATH,
    glam::{self, Vec2, ivec2, vec2},
    rand, rapier3d,
    renderer::renderer::BASE_MATERIAL_INDEX,
    scene::{UiFrame, UiText},
    thunderdome::{self, Index},
    winit,
};

use glam::{EulerRot, Quat, Vec3, vec3};
use rapier3d::{
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
    physics::context::PhysicsContext,
    renderer::renderer::{MeshHandle, VulkanContext},
    scene::{Camera, Lighting, MeshNode, RenderScene},
};

const CAMERA_SPEED: f32 = 100.0;
const STEP_HZ: f32 = 1.0 / 60.0;

struct GameResources {
    cube_mesh: Index,
    sphere_mesh: Index,
    skybox1: Index,
    skybox2: Index,
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
}

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

        let skybox2_id = load_skybox(
            &mut vk_ctx,
            format!(
                "{}/cloudy-skyboxes/Cubemap/Cubemap_Sky_02-512x512.png",
                ASSET_PATH
            )
            .as_str(),
        );

        let resources = GameResources {
            cube_mesh,
            sphere_mesh,
            skybox1: skybox1_id,
            skybox2: skybox2_id,
        };

        let start_time = Instant::now();
        let last_frame = start_time;

        let mut scene = RenderScene::new(
            Camera::new(
                vec3(0.0, 100.0, 100.0),
                Quat::from_euler(EulerRot::ZXY, 0.0, f32::to_radians(-45.0), 0.0),
                90.0,
            ),
            Lighting {
                sun_direction: vec3(0.0, -1.0, -1.0).normalize(),
                sun_color: vec3(1.0, 0.95, 0.85),
                sun_power: 0.5,
                ambient_color: vec3(0.9, 0.95, 1.0) * 0.2,
                skybox_id: skybox1_id,
            },
        );

        // scene.push_ui_frame(UiFrame::new(vec2(0.0, 0.0), vec2(600.0, 300.0), 1));

        scene.push_ui_text(UiText::new(
            vec2(0.0, 0.0),
            32,
            "the quick brown fox doesnt not concern himself with subpixel rendering",
        ));

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

        for _ in 0..200 {
            let cuboid = Part::new_cube(
                &mut physics_context,
                RigidBodyType::Dynamic,
                vec3(
                    rng.random_range(-50.0..50.0),
                    rng.random_range(1.0..50.0),
                    rng.random_range(-50.0..50.0),
                ) * 5.0,
                Quat::from_euler(
                    EulerRot::XYZ,
                    rng.random::<f32>() * std::f32::consts::PI * 2.0,
                    rng.random::<f32>() * std::f32::consts::PI * 2.0,
                    rng.random::<f32>() * std::f32::consts::PI * 2.0,
                ),
                vec3(4.0, 4.0, 4.0) * rng.random_range(1.0..5.0),
                hsv_to_rgb(rng.random::<f32>() * 360.0, 0.8, 1.0),
            );

            cubes.insert(cuboid);
        }

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
        }
    }

    fn update_fixed(&mut self) {
        self.physics_context.step();
    }

    fn move_camera(&mut self, dt: f32, window: &Window) {
        let camera = &mut self.scene.camera;

        let camera_forward = camera.orientation * Vec3::NEG_Z;
        let camera_right = camera.orientation * Vec3::X;

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

        if self.input_state.is_key_down(KeyCode::KeyA) {
            camera.position -= camera_right * dt * CAMERA_SPEED;
        }

        if self.input_state.is_key_down(KeyCode::KeyD) {
            camera.position += camera_right * dt * CAMERA_SPEED;
        }

        if self.input_state.is_key_down(KeyCode::KeyW) {
            camera.position += camera_forward * dt * CAMERA_SPEED;
        }

        if self.input_state.is_key_down(KeyCode::KeyS) {
            camera.position -= camera_forward * dt * CAMERA_SPEED;
        }
    }

    fn handle_spawning_parts(&mut self) {
        let camera_position = self.scene.camera.position;
        let camera_forward = self.scene.camera.orientation * Vec3::NEG_Z;

        if self.input_state.is_key_down(KeyCode::Space) {
            let rng = &mut self.rng;

            let part = if rng.random_bool(0.5) {
                Part::new_cube(
                    &mut self.physics_context,
                    RigidBodyType::Dynamic,
                    camera_position + camera_forward * 30.0,
                    Quat::from_euler(
                        EulerRot::XYZ,
                        rng.random::<f32>() * std::f32::consts::PI * 2.0,
                        rng.random::<f32>() * std::f32::consts::PI * 2.0,
                        rng.random::<f32>() * std::f32::consts::PI * 2.0,
                    ),
                    vec3(4.0, 4.0, 4.0) * rng.random_range(1.0..5.0),
                    hsv_to_rgb(rng.random::<f32>() * 360.0, 0.8, 1.0),
                )
            } else {
                Part::new_sphere(
                    &mut self.physics_context,
                    RigidBodyType::Dynamic,
                    camera_position + camera_forward * 30.0,
                    Quat::from_euler(
                        EulerRot::XYZ,
                        rng.random::<f32>() * std::f32::consts::PI * 2.0,
                        rng.random::<f32>() * std::f32::consts::PI * 2.0,
                        rng.random::<f32>() * std::f32::consts::PI * 2.0,
                    ),
                    rng.random_range(5.0..10.0),
                    hsv_to_rgb(rng.random::<f32>() * 360.0, 0.8, 1.0),
                )
            };

            let rigid_body = self
                .physics_context
                .rigid_body_set
                .get_mut(part.rigid_body_handle)
                .unwrap();
            rigid_body.set_linvel(camera_forward * 500.0, true);
            self.parts.insert(part);
        }
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

        while self.accumulator > STEP_HZ {
            self.update_fixed();
            self.accumulator -= STEP_HZ;
        }

        self.move_camera(dt, window);
        self.handle_spawning_parts();
        self.clean_parts();

        let skybox_switch = ((elapsed / 10.0).floor() as i32) % 10;

        if skybox_switch % 2 == 0 {
            self.scene.lighting.skybox_id = self.resources.skybox1
        } else {
            self.scene.lighting.skybox_id = self.resources.skybox2
        }

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
                        material_id: BASE_MATERIAL_INDEX,
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
                        material_id: BASE_MATERIAL_INDEX,
                    });
                }
            }
        }

        self.vk_ctx.draw(&self.scene);
    }

    pub fn handle_device_event(&mut self, event: &DeviceEvent) {
        match event {
            DeviceEvent::MouseMotion { delta } => {
                self.input_state
                    .mouse_motion((delta.0 as f32, delta.1 as f32));
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
                self.input_state.key_input(event);
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
