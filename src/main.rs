mod mesh;
mod scene;
mod vulkan;

use std::time::SystemTime;

use glam::{EulerRot, Quat, vec3};
use rand::{Rng, SeedableRng, rngs::SmallRng};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes},
};

use vulkan::VulkanContext;

use crate::scene::{Camera, MeshNode, RenderScene};

struct App {
    scene: RenderScene,
    window: Option<Window>,
    vk_ctx: Option<VulkanContext>,
    last_frame_time: SystemTime,
}

impl App {
    fn new() -> Self {
        let last_frame_time = SystemTime::now();

        let mut scene = RenderScene {
            camera: Camera::new(vec3(0.0, 0.0, 5.0), Quat::IDENTITY, 70.0),
            meshes: Vec::new(),
            are_meshes_dirty: true,
        };

        let mut rng = SmallRng::from_os_rng();

        for _ in 0..1000 {
            let instance = MeshNode {
                position: vec3(
                    rng.random_range(-20.0..20.0),
                    rng.random_range(-20.0..20.0),
                    rng.random_range(-20.0..20.0),
                ),
                orientation: Quat::from_euler(
                    EulerRot::XYZ,
                    rng.random::<f32>() * std::f32::consts::PI * 2.0,
                    rng.random::<f32>() * std::f32::consts::PI * 2.0,
                    rng.random::<f32>() * std::f32::consts::PI * 2.0,
                ),
                color: vec3(rng.random(), rng.random(), rng.random()),
                mesh_id: 0,
                material_id: 0,
            };

            scene.meshes.push(instance);
        }

        Self {
            scene,
            last_frame_time,
            window: None,
            vk_ctx: None,
        }
    }

    pub fn update(&mut self) {
        let dt = self.last_frame_time.elapsed().unwrap().as_secs_f32();
        self.last_frame_time = SystemTime::now();
        self.scene.camera.orientation *=
            Quat::from_euler(glam::EulerRot::XYZ, 0.0, f32::to_radians(100.0) * dt, 0.0);
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
        self.update();

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
                }
            }
            WindowEvent::Resized(size) => {
                self.vk_ctx
                    .as_mut()
                    .unwrap()
                    .handle_resize((size.width, size.height));
            }
            WindowEvent::RedrawRequested => {
                self.vk_ctx.as_mut().unwrap().draw(&self.scene);
                self.window.as_ref().unwrap().request_redraw();
                self.scene.are_meshes_dirty = false;
            }
            _ => {}
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App::new();
    event_loop.run_app(&mut app).unwrap();
}
