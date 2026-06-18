use std::time::Instant;

use glam::Vec2;
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::WindowAttributes,
};

use crate::{game::GameContext, input::InputEvent};

const STEP_HZ: f32 = 1.0 / 60.0;

pub trait GameHandler {
    fn new(game_ctx: &mut GameContext) -> Self;

    fn fixed_update(&mut self, game_ctx: &mut GameContext, dt: f32) {}
    fn update(&mut self, game_ctx: &mut GameContext, dt: f32) {}
    fn draw(&mut self, game_ctx: &mut GameContext) {}

    fn on_input(&mut self, game_ctx: &mut GameContext, input_event: InputEvent) {}
}

pub struct InitializedGameApp<T: GameHandler> {
    game_ctx: GameContext,
    game_handler: T,
}

impl<T: GameHandler> InitializedGameApp<T> {
    fn update(&mut self) {
        let now = Instant::now();
        let dt = (now - self.game_ctx.last_frame).as_secs_f32();
        self.game_ctx.accumulator += dt;

        self.game_ctx.last_frame = now;
        self.game_handler.update(&mut self.game_ctx, dt);

        while self.game_ctx.accumulator > STEP_HZ {
            self.game_handler.fixed_update(&mut self.game_ctx, STEP_HZ);
            self.game_ctx.accumulator -= STEP_HZ;
        }

        let inner_size = self.game_ctx.window.inner_size();

        self.game_ctx
            .ui_tree
            .set_root_size(Vec2::new(inner_size.width as f32, inner_size.height as f32));

        self.game_ctx
            .ui_tree
            .handle_input(&self.game_ctx.input_state);

        // PERF: find a way to avoid cloning
        for input in self
            .game_ctx
            .input_state
            .get_input_events()
            .to_owned()
            .into_iter()
        {
            self.game_handler.on_input(&mut self.game_ctx, input);
        }

        self.game_ctx.input_state.clear();
    }

    fn draw(&mut self) {
        self.game_ctx.render_scene.ui.clear();

        {
            let mut font_manager = self.game_ctx.font_manager.lock().unwrap();
            self.game_ctx
                .ui_tree
                .draw(&mut font_manager, &mut self.game_ctx.render_scene.ui);
        }

        self.game_handler.draw(&mut self.game_ctx);
        self.game_ctx.vk_ctx.draw(&self.game_ctx.render_scene);
    }
}

pub enum GameApp<T: GameHandler> {
    Uninitialized,
    Initialized(InitializedGameApp<T>),
}

impl<T: GameHandler> GameApp<T> {
    fn new() -> Self {
        GameApp::Uninitialized
    }

    fn unwrap_initialized(&mut self) -> Option<&mut InitializedGameApp<T>> {
        match self {
            GameApp::Uninitialized => None,
            GameApp::Initialized(initialized_game_app) => Some(initialized_game_app),
        }
    }
}

impl<T: GameHandler> ApplicationHandler for GameApp<T> {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if let GameApp::Initialized { .. } = self {
            return;
        }

        let window = event_loop
            .create_window(WindowAttributes::default())
            .unwrap();

        let mut game_ctx = GameContext::new(window);

        let game_handler = T::new(&mut game_ctx);

        *self = GameApp::Initialized(InitializedGameApp {
            game_ctx,
            game_handler,
        })
    }

    fn device_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: DeviceEvent,
    ) {
        let app = self.unwrap_initialized().unwrap();
        match event {
            DeviceEvent::MouseMotion { delta } => {
                app.game_ctx
                    .input_state
                    .mouse_motion((delta.0 as f32, delta.1 as f32));
            }
            DeviceEvent::Key(key_event) => {
                app.game_ctx.input_state.raw_key_input(&key_event);
            }
            _ => {}
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let app = self.unwrap_initialized().unwrap();

        let mut should_draw = false;

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            WindowEvent::KeyboardInput {
                device_id: _,
                ref event,
                is_synthetic: _,
            } => {
                if PhysicalKey::Code(KeyCode::Escape) == event.physical_key {
                    event_loop.exit();
                    return;
                }
            }

            WindowEvent::MouseInput {
                device_id: _,
                state,
                button,
            } => {
                app.game_ctx.input_state.mouse_input(&button, &state);
            }

            WindowEvent::CursorMoved {
                device_id: _,
                position,
            } => {
                app.game_ctx.input_state.mouse_moved(&position);
            }

            WindowEvent::Resized(size) => {
                app.game_ctx.vk_ctx.handle_resize((size.width, size.height));
            }

            WindowEvent::RedrawRequested => {
                should_draw = true;
                app.game_ctx.window.request_redraw();
            }
            _ => {}
        }

        app.update();

        if should_draw {
            app.draw();
        }
    }
}

pub fn start_game_app<T: GameHandler>() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = GameApp::<T>::new();
    event_loop.run_app(&mut app).unwrap();
}
