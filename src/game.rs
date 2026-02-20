use std::time::Instant;

use ash::vk;
use glam::{EulerRot, Quat, Vec2, Vec3, vec3};
use image::{EncodableLayout, GenericImage};
use rand::{SeedableRng, rngs::SmallRng};
use winit::{
    event::{DeviceEvent, WindowEvent},
    keyboard::KeyCode,
    window::Window,
};

use crate::{
    input::InputState,
    renderer::{
        mesh::MeshVertex,
        renderer::{MeshHandle, SkyboxImageData, VulkanContext},
    },
    scene::{Camera, Lighting, MeshData, MeshNode, RenderScene},
};

const CAMERA_SPEED: f32 = 100.0;

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

pub struct Game {
    vk_ctx: VulkanContext,
    input_state: InputState,
    scene: RenderScene,
    last_frame: Instant,
    start_time: Instant,
    rng: SmallRng,
    resources: GameResources,
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
                sun_direction: vec3(0.0, -1.0, 0.0),
                sun_color: vec3(1.0, 0.95, 0.85),
                sun_power: 0.5,
                ambient_color: vec3(0.9, 0.95, 1.0) * 0.2,
                skybox_id: skybox1_id,
            },
        };

        let mut rng = SmallRng::from_os_rng();

        scene.meshes.push(MeshNode {
            position: vec3(0.0, -25.0, 0.0),
            orientation: Quat::IDENTITY,
            size: vec3(512.0, 50.0, 512.0),
            color: vec3(0.8, 0.8, 0.8),
            opacity: 1.0,
            mesh_id: resources.cube_mesh,
            material_id: 1,
        });

        let input_state = InputState::new();

        Self {
            vk_ctx,
            scene,
            last_frame,
            start_time,
            input_state,
            rng,
            resources,
        }
    }

    pub fn update(&mut self, window: &Window) {
        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32();
        let elapsed = (now - self.start_time).as_secs_f32();

        let skybox_switch = ((elapsed / 10.0).floor() as i32) % 10;

        if skybox_switch % 2 == 0 {
            self.scene.lighting.skybox_id = self.resources.skybox1
        } else {
            self.scene.lighting.skybox_id = self.resources.skybox2
        }

        self.last_frame = now;

        let sun_dir = vec3(elapsed.cos(), -1.0, elapsed.sin()).normalize();
        self.scene.lighting.sun_direction = sun_dir;

        let camera = &mut self.scene.camera;

        let forward = camera.orientation * Vec3::NEG_Z;
        let right = camera.orientation * Vec3::X;

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
            camera.position -= right * dt * CAMERA_SPEED;
        }

        if self.input_state.is_key_down(KeyCode::KeyD) {
            camera.position += right * dt * CAMERA_SPEED;
        }

        if self.input_state.is_key_down(KeyCode::KeyW) {
            camera.position += forward * dt * CAMERA_SPEED;
        }

        if self.input_state.is_key_down(KeyCode::KeyS) {
            camera.position -= forward * dt * CAMERA_SPEED;
        }

        self.input_state.clear();
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
            WindowEvent::RedrawRequested => {
                self.vk_ctx.draw(&self.scene);
            }
            _ => {}
        }
    }
}
