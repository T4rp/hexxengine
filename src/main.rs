mod color;
mod input;
mod mesh;
mod scene;
mod vulkan;

use std::time::Instant;

use glam::{EulerRot, Quat, Vec2, Vec3, vec3};
use gltf::Mesh;
use rand::{Rng, SeedableRng, rngs::SmallRng};
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes},
};

use vulkan::VulkanContext;

use crate::{
    color::hsv_to_rgb,
    input::InputState,
    mesh::MeshVertex,
    scene::{Camera, Lighting, MeshData, MeshNode, RenderScene},
};

const CAMERA_SPEED: f32 = 100.0;

struct App {
    scene: RenderScene,
    window: Option<Window>,
    vk_ctx: Option<VulkanContext>,
    last_frame: Instant,
    start_time: Instant,
    input_state: InputState,
}

fn process_gltf_mesh(mesh: &Mesh, buffers: &[gltf::buffer::Data]) -> MeshData {
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

impl App {
    fn new() -> Self {
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
            },
        };

        let mut rng = SmallRng::from_os_rng();

        scene.meshes.push(MeshNode {
            position: vec3(0.0, -25.0, 0.0),
            orientation: Quat::IDENTITY,
            size: vec3(512.0, 50.0, 512.0),
            color: vec3(0.8, 0.8, 0.8),
            opacity: 1.0,
            mesh_id: 0,
            material_id: 1,
        });

        for _ in 0..200 {
            let opacity = if rng.random_bool(0.75) { 1.0 } else { 0.5 };
            let instance = MeshNode {
                position: vec3(
                    rng.random_range(-50.0..50.0),
                    rng.random_range(1.0..50.0),
                    rng.random_range(-50.0..50.0),
                ) * 5.0,
                orientation: Quat::from_euler(
                    EulerRot::XYZ,
                    rng.random::<f32>() * std::f32::consts::PI * 2.0,
                    rng.random::<f32>() * std::f32::consts::PI * 2.0,
                    rng.random::<f32>() * std::f32::consts::PI * 2.0,
                ),
                size: vec3(4.0, 4.0, 4.0) * rng.random_range(1.0..5.0),
                color: hsv_to_rgb(rng.random::<f32>() * 360.0, 0.8, 1.0),
                opacity: opacity,
                mesh_id: rng.random_range(0..=1),
                material_id: rng.random_range(0..=1),
            };

            scene.meshes.push(instance);
        }

        let input_state = InputState::new();

        Self {
            scene,
            last_frame,
            start_time,
            window: None,
            vk_ctx: None,
            input_state,
        }
    }

    pub fn init_vk(&mut self) {
        let vk_ctx = self.vk_ctx.as_mut().unwrap();

        let cube_mesh = get_first_gltf_mesh("./assets/cube.gltf");
        let sphere_mesh = get_first_gltf_mesh("./assets/sphere.gltf");

        vk_ctx.load_mesh(&cube_mesh.vertices, &cube_mesh.indices);
        vk_ctx.load_mesh(&sphere_mesh.vertices, &sphere_mesh.indices);
    }

    pub fn update(&mut self) {
        let window = self.window.as_ref().unwrap();

        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32();
        let elapsed = (now - self.start_time).as_secs_f32();

        self.last_frame = now;

        let sun_dir = vec3(elapsed.cos(), -1.0, elapsed.sin()).normalize();
        self.scene.lighting.sun_direction = sun_dir;

        let last_mesh = self.scene.meshes.last_mut().unwrap();
        last_mesh.orientation = Quat::IDENTITY;
        last_mesh.position = -sun_dir * 30.0;

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
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = event_loop
            .create_window(WindowAttributes::default())
            .unwrap();

        let vk_ctx = VulkanContext::new(&window);

        self.window = Some(window);
        self.vk_ctx = Some(vk_ctx);

        self.init_vk();
    }

    fn device_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: DeviceEvent,
    ) {
        match event {
            DeviceEvent::MouseMotion { delta } => {
                self.input_state
                    .mouse_motion((delta.0 as f32, delta.1 as f32));
            }
            _ => {}
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let mut should_draw = false;

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => {
                if PhysicalKey::Code(KeyCode::Escape) == event.physical_key {
                    event_loop.exit();
                    return;
                }
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
                self.vk_ctx
                    .as_mut()
                    .unwrap()
                    .handle_resize((size.width, size.height));
            }
            WindowEvent::RedrawRequested => {
                self.window.as_ref().unwrap().request_redraw();
                should_draw = true;
            }
            _ => {}
        }

        self.update();

        if should_draw {
            self.vk_ctx.as_mut().unwrap().draw(&self.scene);
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App::new();
    event_loop.run_app(&mut app).unwrap();
}
