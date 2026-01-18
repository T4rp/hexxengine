mod mesh;
mod vulkan;

use std::rc::Rc;

use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes},
};

use vulkan::VulkanContext;

struct App {
    window: Option<Rc<Window>>,
    vk_ctx: Option<VulkanContext>,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            vk_ctx: None,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = Rc::new(
            event_loop
                .create_window(WindowAttributes::default())
                .unwrap(),
        );

        let vk_ctx = VulkanContext::new(window.clone());

        self.window = Some(window);
        self.vk_ctx = Some(vk_ctx);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
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
            WindowEvent::Resized(_size) => {
                self.vk_ctx.as_mut().unwrap().handle_resize();
            }
            WindowEvent::RedrawRequested => {
                self.vk_ctx.as_mut().unwrap().update();
                self.vk_ctx.as_mut().unwrap().draw();
                self.window.as_ref().unwrap().request_redraw();
            }
            _ => {}
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut app = App::new();
    event_loop.run_app(&mut app).unwrap();
}
