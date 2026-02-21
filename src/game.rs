use std::time::Instant;

use ash::vk;
use glam::{EulerRot, Quat, Vec2, Vec3, vec3};
use image::{EncodableLayout, GenericImage};
use rand::{Rng, SeedableRng, rngs::SmallRng};
use rapier3d::{
    math::{Pose, Pose3},
    na::vector,
    parry::simba::scalar::SupersetOf,
    prelude::{
        CCDSolver, ColliderBuilder, ColliderHandle, ColliderSet, DefaultBroadPhase,
        ImpulseJointSet, IntegrationParameters, IslandManager, MultibodyJointSet, NarrowPhase,
        PhysicsPipeline, RigidBodyBuilder, RigidBodyHandle, RigidBodySet,
    },
};
use winit::{
    event::{DeviceEvent, WindowEvent},
    keyboard::KeyCode,
    window::Window,
};

use crate::{
    color::hsv_to_rgb,
    input::InputState,
    renderer::{
        mesh::MeshVertex,
        renderer::{MeshHandle, SkyboxImageData, VulkanContext},
    },
    scene::{Camera, Lighting, MeshData, MeshNode, RenderScene},
};

const CAMERA_SPEED: f32 = 100.0;
const STEP_HZ: f32 = 1.0 / 60.0;

fn process_gltf_mesh(mesh: &gltf::Mesh, buffers: &[gltf::buffer::Data]) -> MeshData {
    let mut mesh_vertices = Vec::new();
    let mut mesh_indices = Vec::new();

    let prim = mesh.primitives().next().unwrap();
    let reader = prim.reader(|b| Some(&buffers[b.index()]));

    let mut positions = reader.read_positions().unwrap();
    let mut normals = reader.read_normals().unwrap();
    let mut uvs = reader.read_tex_coords(0).unwrap().into_f32();
    let indices = reader.read_indices().unwrap().into_u32();

    let v_count = positions.len();

    for _ in 0..v_count {
        let position = positions.next().unwrap();
        let normal = normals.next().unwrap();
        let uv = uvs.next().unwrap();

        mesh_vertices.push(MeshVertex {
            pos: Vec3::from_slice(&position),
            norm: Vec3::from_slice(&normal),
            uv: Vec2::from_slice(&uv),
        });
    }

    for index in indices {
        mesh_indices.push(index as u16);
    }

    MeshData {
        vertices: mesh_vertices,
        indices: mesh_indices,
    }
}

fn get_first_gltf_mesh(filename: &str) -> MeshData {
    let (gltf, buffers, _images) = gltf::import(filename).unwrap();

    let mesh = gltf.meshes().next().unwrap();
    process_gltf_mesh(&mesh, &buffers)
}

fn load_skybox<'a>(render: &mut VulkanContext, file_path: &str) -> u32 {
    let mut skybox_image = image::open(file_path).unwrap().into_rgba8();

    let top_image = skybox_image.sub_image(512, 0, 512, 512).to_image();
    let left_image = skybox_image.sub_image(0, 512, 512, 512).to_image();
    let back_image = skybox_image.sub_image(512, 512, 512, 512).to_image();
    let right_image = skybox_image.sub_image(512 * 2, 512, 512, 512).to_image();
    let front_image = skybox_image.sub_image(512 * 3, 512, 512, 512).to_image();
    let bottom_image = skybox_image.sub_image(512, 512 * 2, 512, 512).to_image();

    let skybox_data = SkyboxImageData {
        width: 512,
        height: 512,
        top: top_image.as_bytes(),
        bottom: bottom_image.as_bytes(),
        front: front_image.as_bytes(),
        back: back_image.as_bytes(),
        left: left_image.as_bytes(),
        right: right_image.as_bytes(),
    };

    render.load_skybox(vk::Filter::LINEAR, &skybox_data)
}

struct GameResources {
    cube_mesh: MeshHandle,
    sphere_mesh: MeshHandle,
    skybox1: u32,
    skybox2: u32,
}

struct PhysicsContext {
    gravity: Vec3,
    rigid_body_set: RigidBodySet,
    collider_set: ColliderSet,
    impulse_joint_set: ImpulseJointSet,
    multibody_joint_set: MultibodyJointSet,
    integration_parameters: IntegrationParameters,
    island_manager: IslandManager,
    broad_phase: DefaultBroadPhase,
    narrow_phase: NarrowPhase,
    ccd_solver: CCDSolver,
    physics_pipeline: PhysicsPipeline,
}

impl PhysicsContext {
    fn new() -> Self {
        let mut rigid_body_set = RigidBodySet::new();
        let mut collider_set = ColliderSet::new();
        let mut impulse_joint_set = ImpulseJointSet::new();
        let mut multibody_joint_set = MultibodyJointSet::new();

        let gravity = vec3(0.0, -196.0, 0.0);
        let integration_parameters = IntegrationParameters {
            length_unit: 1.0,
            ..Default::default()
        };
        let mut physics_pipeline = PhysicsPipeline::new();
        let mut island_manager = IslandManager::new();
        let mut broad_phase = DefaultBroadPhase::new();
        let mut narrow_phase = NarrowPhase::new();
        let mut ccd_solver = CCDSolver::new();

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

    fn step(&mut self) {
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

pub struct Game {
    vk_ctx: VulkanContext,
    input_state: InputState,
    scene: RenderScene,
    last_frame: Instant,
    start_time: Instant,
    rng: SmallRng,
    resources: GameResources,
    cubes: Vec<Cuboid>,
    physics_context: PhysicsContext,
    accumulator: f32,
}

struct Cuboid {
    position: Vec3,
    orientation: Quat,
    size: Vec3,
    color: Vec3,
    collider: ColliderHandle,
    rigid_body_handle: Option<RigidBodyHandle>,
}

impl Cuboid {
    fn new(
        collider_set: &mut ColliderSet,
        position: Vec3,
        orientation: Quat,
        size: Vec3,
        color: Vec3,
    ) -> Self {
        let collider = ColliderBuilder::cuboid(size.x / 2.0, size.y / 2.0, size.z / 2.0)
            .position(Pose3::from_parts(position, orientation))
            .build();
        let collider_handle = collider_set.insert(collider);

        Self {
            position,
            orientation,
            size,
            color,
            collider: collider_handle,
            rigid_body_handle: None,
        }
    }

    fn new_rigid_body(
        collider_set: &mut ColliderSet,
        rigid_body_set: &mut RigidBodySet,
        position: Vec3,
        orientation: Quat,
        size: Vec3,
        color: Vec3,
    ) -> Self {
        let collider = ColliderBuilder::cuboid(size.x / 2.0, size.y / 2.0, size.z / 2.0).build();

        let rigid_body = RigidBodyBuilder::dynamic()
            .pose(Pose3::from_parts(position, orientation))
            .build();

        let rigid_body_handle = rigid_body_set.insert(rigid_body);

        let collider_handle =
            collider_set.insert_with_parent(collider, rigid_body_handle, rigid_body_set);

        Self {
            position,
            orientation,
            size,
            color,
            collider: collider_handle,
            rigid_body_handle: Some(rigid_body_handle),
        }
    }
}

impl Game {
    pub fn new(window: &Window) -> Self {
        let mut vk_ctx = VulkanContext::new(&window);

        let cube_mesh = get_first_gltf_mesh("./assets/cube.gltf");
        let sphere_mesh = get_first_gltf_mesh("./assets/sphere.gltf");

        let cube_mesh = vk_ctx.load_mesh(&cube_mesh.vertices, &cube_mesh.indices);
        let sphere_mesh = vk_ctx.load_mesh(&sphere_mesh.vertices, &sphere_mesh.indices);

        let skybox1_id = load_skybox(
            &mut vk_ctx,
            "assets/cloudy-skyboxes/Cubemap/Cubemap_Sky_04-512x512.png",
        );

        let skybox2_id = load_skybox(
            &mut vk_ctx,
            "assets/cloudy-skyboxes/Cubemap/Cubemap_Sky_02-512x512.png",
        );

        let resources = GameResources {
            cube_mesh,
            sphere_mesh,
            skybox1: skybox1_id,
            skybox2: skybox2_id,
        };

        let start_time = Instant::now();
        let last_frame = start_time;

        let mut scene = RenderScene {
            camera: Camera::new(
                vec3(0.0, 100.0, 100.0),
                Quat::from_euler(EulerRot::ZXY, 0.0, f32::to_radians(-45.0), 0.0),
                90.0,
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

        let mut cubes = Vec::new();

        cubes.push(Cuboid::new(
            &mut physics_context.collider_set,
            vec3(0.0, -25.0, 0.0),
            Quat::IDENTITY,
            vec3(512.0, 50.0, 512.0),
            vec3(0.8, 0.8, 0.8),
        ));

        for _ in 0..200 {
            let cuboid = Cuboid::new_rigid_body(
                &mut physics_context.collider_set,
                &mut physics_context.rigid_body_set,
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

            cubes.push(cuboid);
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
            cubes,
            physics_context,
            accumulator: 0.0,
        }
    }

    fn update_fixed(&mut self, window: &Window) {
        self.physics_context.step();
    }

    pub fn update(&mut self, window: &Window) {
        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32();
        let elapsed = (now - self.start_time).as_secs_f32();

        self.accumulator += dt;

        while self.accumulator > STEP_HZ {
            self.update_fixed(window);
            self.accumulator -= STEP_HZ;
        }

        let camera = &mut self.scene.camera;

        let camera_forward = camera.orientation * Vec3::NEG_Z;
        let camera_right = camera.orientation * Vec3::X;

        if self.input_state.is_key_pressed(KeyCode::Space) {
            let rng = &mut self.rng;

            let cuboid = Cuboid::new_rigid_body(
                &mut self.physics_context.collider_set,
                &mut self.physics_context.rigid_body_set,
                camera.position + camera_forward * 30.0,
                Quat::from_euler(
                    EulerRot::XYZ,
                    rng.random::<f32>() * std::f32::consts::PI * 2.0,
                    rng.random::<f32>() * std::f32::consts::PI * 2.0,
                    rng.random::<f32>() * std::f32::consts::PI * 2.0,
                ),
                vec3(4.0, 4.0, 4.0) * rng.random_range(1.0..5.0),
                hsv_to_rgb(rng.random::<f32>() * 360.0, 0.8, 1.0),
            );

            let rigid_body_handle = cuboid.rigid_body_handle.unwrap();
            let rigit_body = self
                .physics_context
                .rigid_body_set
                .get_mut(rigid_body_handle)
                .unwrap();
            rigit_body.set_linvel(camera_forward * 500.0, true);
            self.cubes.push(cuboid);
        }

        for cube in self.cubes.iter_mut() {
            let Some(rigid_body_handle) = cube.rigid_body_handle else {
                continue;
            };

            let rigid_body = self
                .physics_context
                .rigid_body_set
                .get(rigid_body_handle)
                .unwrap();

            let pose = rigid_body.position();
            cube.position = pose.translation;
            cube.orientation = pose.rotation;
        }

        let skybox_switch = ((elapsed / 10.0).floor() as i32) % 10;

        if skybox_switch % 2 == 0 {
            self.scene.lighting.skybox_id = self.resources.skybox1
        } else {
            self.scene.lighting.skybox_id = self.resources.skybox2
        }

        self.last_frame = now;

        let sun_dir = vec3(elapsed.cos(), -1.0, elapsed.sin()).normalize();
        // self.scene.lighting.sun_direction = sun_dir;

        let mouse_delta = self.input_state.mouse_delta;

        if self.input_state.right_mouse_down {
            let _ = window
                .set_cursor_grab(winit::window::CursorGrabMode::Confined)
                .or_else(|_| window.set_cursor_grab(winit::window::CursorGrabMode::Locked));
            window.set_cursor_visible(false);
        } else {
            let _ = window.set_cursor_grab(winit::window::CursorGrabMode::None);
            window.set_cursor_visible(true);
        }

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

        self.input_state.clear();
    }

    fn draw(&mut self) {
        self.scene.meshes.clear();

        for cube in self.cubes.iter() {
            self.scene.meshes.push(MeshNode {
                position: cube.position,
                orientation: cube.orientation,
                size: cube.size,
                color: cube.color,
                opacity: 1.0,
                mesh_id: self.resources.cube_mesh,
                material_id: 1,
            });
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
