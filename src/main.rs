mod color;
mod input;
mod mat4ext;
mod mesh;
mod scene;
mod vulkan;

use std::time::Instant;

use glam::{EulerRot, Quat, Vec3, vec3};
use rand::{Rng, SeedableRng, rngs::SmallRng};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalPosition,
    event::{ElementState, KeyEvent, MouseButton, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes},
};

use vulkan::VulkanContext;

use crate::{
    color::hsv_to_rgb,
    input::InputState,
    scene::{Camera, Lighting, MeshNode, RenderScene},
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

impl App {
    fn new() -> Self {
        let start_time = Instant::now();
        let last_frame = start_time.clone();

        let mut scene = RenderScene {
            camera: Camera::new(
                vec3(0.0, 100.0, 100.0),
                Quat::from_euler(EulerRot::ZXY, 0.0, f32::to_radians(-45.0), 0.0),
                90.0,
            ),
            meshes: Vec::new(),
            lighting: Lighting {
                sun_direction: vec3(0.0, -1.0, 0.0),
                sun_color: vec3(1.0, 1.0, 1.0),
                sun_power: 0.5,
                ambient_color: vec3(1.0, 1.0, 1.0) * 0.01,
            },
            are_meshes_dirty: true,
        };

        let mut rng = SmallRng::from_os_rng();

        scene.meshes.push(MeshNode {
            position: vec3(0.0, -25.0, 0.0),
            orientation: Quat::IDENTITY,
            size: vec3(512.0, 50.0, 512.0),
            color: vec3(0.8, 0.8, 0.8),
            mesh_id: 0,
            material_id: 0,
        });

        for _ in 0..200 {
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
                mesh_id: rng.random_range(0..=1),
                material_id: 0,
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

    pub fn update(&mut self) {
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
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let mut should_draw = false;

        self.input_state.clear();

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
                device_id,
                state,
                button,
            } => {
                self.input_state.mouse_input(button, state);
            }
            WindowEvent::CursorMoved {
                device_id,
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
