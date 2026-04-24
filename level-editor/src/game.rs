use std::time::Instant;

use hexxengine::{
    ash::vk,
    assets::{ASSET_PATH, get_first_gltf_mesh, load_skybox},
    components::{MeshComponent, TransformComponent},
    entities::Part,
    glam::{EulerRot, Quat, Vec3, vec3},
    input::InputState,
    physics::context::PhysicsContext,
    rapier3d::prelude::{RigidBodyType, ShapeType},
    renderer::renderer::{BASE_MATERIAL_INDEX, FALLBACK_SKYBOX_INDEX, VulkanContext},
    scene::{Camera, Lighting, MeshNode, RenderScene},
    thunderdome::Arena,
    winit::{
        self,
        event::{DeviceEvent, WindowEvent},
        keyboard::KeyCode,
        window::Window,
    },
};

const CAMERA_SPEED: f32 = 100.0;
const CAMERA_SENSITIVITY: f32 = 0.38;
const STEP_HZ: f32 = 1.0 / 60.0;

pub struct World {
    parts: Arena<Part>,
}

impl World {
    fn new() -> Self {
        Self {
            parts: Arena::new(),
        }
    }
}

pub struct Game {
    vk_ctx: VulkanContext,
    input_state: InputState,
    render_scene: RenderScene,
    world: World,
    start_time: Instant,
    last_frame: Instant,
    physics_context: PhysicsContext,
    accumulator: f32,
}

impl Game {
    pub fn new(window: &Window) -> Self {
        let mut vk_ctx = VulkanContext::new(&window);
        let input_state = InputState::new();
        let start_time = Instant::now();

        let cube_mesh = get_first_gltf_mesh(format!("{}/cube.gltf", ASSET_PATH).as_str());
        let cube_mesh_id = vk_ctx.load_mesh(&cube_mesh.vertices, &cube_mesh.indices);

        let skybox = load_skybox(
            &mut vk_ctx,
            format!(
                "{}/cloudy-skyboxes/Cubemap/Cubemap_Sky_04-512x512.png",
                ASSET_PATH
            )
            .as_str(),
        );

        let mut render_scene = RenderScene::new(
            Camera::new(
                vec3(0.0, 100.0, 100.0),
                Quat::from_euler(EulerRot::ZXY, 0.0, f32::to_radians(-45.0), 0.0),
                120.0,
            ),
            Lighting {
                sun_direction: vec3(0.0, -1.0, -1.0).normalize(),
                sun_color: vec3(1.0, 0.95, 0.85),
                sun_power: 0.5,
                ambient_color: vec3(0.9, 0.95, 1.0) * 0.2,
                skybox_id: FALLBACK_SKYBOX_INDEX,
            },
        );

        render_scene.lighting.skybox_id = skybox;

        let mut physics_context = PhysicsContext::new();

        let mut world = World::new();

        let baseplate = Part::new(
            &mut physics_context,
            TransformComponent {
                position: vec3(0.0, -25.0, 0.0),
                orientation: Quat::IDENTITY,
                size: vec3(2048.0, 50.0, 2048.0),
            },
            MeshComponent {
                color: vec3(0.2, 0.2, 0.2),
                mesh_id: cube_mesh_id,
                material: BASE_MATERIAL_INDEX,
                opacity: 1.0,
            },
            RigidBodyType::Fixed,
            ShapeType::Cuboid,
        );

        world.parts.insert(baseplate);

        Game {
            vk_ctx,
            input_state,
            render_scene,
            start_time,
            physics_context,
            last_frame: start_time,
            world,
            accumulator: 0.0,
        }
    }

    fn update_camera(&mut self, dt: f32, window: &Window) {
        let camera = &mut self.render_scene.camera;

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
            let sensitivity = 0.002 * CAMERA_SENSITIVITY;

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

    fn fixed_update(&mut self) {}

    fn update(&mut self, window: &Window) {
        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32();
        let elapsed = (now - self.start_time).as_secs_f32();
        self.last_frame = now;

        self.update_camera(dt, window);

        while self.accumulator > STEP_HZ {
            self.fixed_update();
            self.accumulator -= STEP_HZ;
        }

        self.input_state.clear();
    }

    fn draw(&mut self) {
        self.render_scene.meshes.clear();

        for (_i, part) in self.world.parts.iter() {
            self.render_scene.meshes.push(MeshNode {
                position: part.transform.position,
                orientation: part.transform.orientation,
                size: part.transform.size,
                color: part.mesh.color,
                opacity: part.mesh.opacity,
                mesh_id: part.mesh.mesh_id,
                material_id: part.mesh.material,
            });
        }

        self.vk_ctx.draw(&self.render_scene);
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

    pub fn handle_window_event(&mut self, window: &Window, window_event: &WindowEvent) {
        let mut should_draw = false;

        match window_event {
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
                should_draw = true;
                window.request_redraw();
            }
            _ => {}
        }

        self.update(window);

        if should_draw {
            self.draw();
        }
    }
}
