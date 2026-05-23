use std::{
    fs,
    sync::{Arc, Mutex},
    time::Instant,
};

use hexxengine::{
    assets::ASSET_PATH,
    components::{MeshComponent, RigidBodyComponent, TransformComponent},
    entities::Part,
    glam::{self, Vec2},
    rand,
    rapier3d::{
        self,
        prelude::ShapeType,
    },
    renderer::renderer::BASE_MATERIAL_INDEX,
    scene::UiText,
    text::{FontHandle, FontManager, TextBox},
    thunderdome::{self, Index},
    winit,
};

use glam::{EulerRot, Quat, Vec3, vec3};
use rapier3d::prelude::RigidBodyType;

use rand::{Rng, SeedableRng, rngs::SmallRng};
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
    renderer::renderer::VulkanContext,
    scene::{Camera, Lighting, MeshNode, RenderScene},
};

use crate::{
    components::CharacterControllerComponent,
    entities::{Character, World},
};

const FRAMERATE_LIMIT_HZ: f32 = 1.0 / 80.0;

const CAMERA_SENSITIVITY: f32 = 0.38;

const STEP_HZ: f32 = 1.0 / 60.0;
const CAMERA_SPEED: f32 = 100.0;
const CHARACTER_HEIGHT: f32 = 10.0;
const CHARACTER_RADIUS: f32 = 2.0;

struct GameResources {
    font: FontHandle,
    cube_mesh: Index,
    sphere_mesh: Index,
    skybox1: Index,
}

pub struct Game {
    font_manager: Arc<Mutex<FontManager>>,
    vk_ctx: VulkanContext,
    input_state: InputState,
    scene: RenderScene,
    last_frame: Instant,
    start_time: Instant,
    rng: SmallRng,
    resources: GameResources,
    world: World,
    physics_context: PhysicsContext,
    accumulator: f32,
    draw_accumulator: f32,

    speed_textbox: TextBox,
}

impl Game {
    pub fn new(window: &Window) -> Self {
        let mut font_manager = FontManager::new();
        let font_data = fs::read(format!("{}/unifont-17.0.03.otf", ASSET_PATH)).unwrap();
        let font_handle = font_manager.load_font(&font_data).unwrap();

        let font_manager = Arc::new(Mutex::new(font_manager));
        let mut vk_ctx = VulkanContext::new(&window, font_manager.clone());

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
            font: font_handle,
            cube_mesh,
            sphere_mesh,
            skybox1: skybox1_id,
        };

        let start_time = Instant::now();
        let last_frame = start_time;

        let scene = RenderScene::new(
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
                skybox_id: skybox1_id,
            },
        );

        let mut rng = SmallRng::from_os_rng();
        let mut physics_context = PhysicsContext::new();
        let input_state = InputState::new();

        let mut world = World::new();

        let baseplate_transform = TransformComponent {
            position: vec3(0.0, -25.0, 0.0),
            orientation: Quat::IDENTITY,
            size: vec3(2048.0, 50.0, 2048.0),
        };

        let baseplate = Part {
            transform: baseplate_transform,
            mesh: MeshComponent {
                color: vec3(0.2, 0.2, 0.2),
                mesh_id: resources.cube_mesh,
                material: BASE_MATERIAL_INDEX,
                opacity: 1.0,
            },
            rigid_body: RigidBodyComponent::new(
                &mut physics_context,
                &baseplate_transform,
                RigidBodyType::Fixed,
                ShapeType::Cuboid,
            ),
        };

        world.parts.insert(baseplate);

        for _ in 0..200 {
            let transform = TransformComponent {
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
            };

            let part = Part {
                transform,
                mesh: MeshComponent {
                    color: hsv_to_rgb(rng.random::<f32>() * 360.0, 0.8, 1.0),
                    mesh_id: resources.cube_mesh,
                    material: BASE_MATERIAL_INDEX,
                    opacity: 1.0,
                },
                rigid_body: RigidBodyComponent::new(
                    &mut physics_context,
                    &transform,
                    RigidBodyType::Dynamic,
                    ShapeType::Cuboid,
                ),
            };

            world.parts.insert(part);
        }

        let character_transform = TransformComponent {
            position: vec3(0.0, 25.0, 25.0),
            orientation: Quat::IDENTITY,
            size: vec3(
                CHARACTER_RADIUS * 2.0,
                CHARACTER_HEIGHT,
                CHARACTER_RADIUS * 2.0,
            ),
        };

        let character = Character {
            transform: character_transform,
            mesh: MeshComponent {
                color: Vec3::ZERO,
                mesh_id: resources.cube_mesh,
                material: BASE_MATERIAL_INDEX,
                opacity: 1.0,
            },
            controller: CharacterControllerComponent::new(
                &mut physics_context,
                &character_transform,
            ),
        };

        let character_handle = world.characters.insert(character);
        world.character_index = Some(character_handle);

        let mut speed_textbox = TextBox::new(font_handle);
        speed_textbox.font_height = 32;

        Self {
            vk_ctx,
            scene,
            last_frame,
            start_time,
            input_state,
            rng,
            resources,
            world,
            physics_context,
            accumulator: 0.0,
            draw_accumulator: 0.0,
            font_manager,
            speed_textbox,
        }
    }

    fn update_fixed(&mut self) {
        let character = self
            .world
            .characters
            .get_mut(self.world.character_index.unwrap())
            .unwrap();

        character
            .controller
            .move_dir(&mut self.physics_context, STEP_HZ);

        self.physics_context.step();
    }

    fn update_character_movement(&mut self, dt: f32) {
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

        let character = self
            .world
            .characters
            .get_mut(self.world.character_index.unwrap())
            .unwrap();

        if self.input_state.is_key_down(KeyCode::Space) {
            character.controller.jump = true;
        }

        character
            .controller
            .update_position(dt, &mut character.transform);

        let mut world_move = self.scene.camera.orientation * move_dir;
        world_move.y = 0.0;

        world_move = world_move.normalize_or_zero();

        character.controller.move_dir = world_move;
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
            let sensitivity = 0.002 * CAMERA_SENSITIVITY;

            let yaw = Quat::from_rotation_y(-mouse_delta.x * sensitivity);
            let pitch = Quat::from_rotation_x(-mouse_delta.y * sensitivity);

            camera.orientation = yaw * camera.orientation * pitch;
        }

        let character = match self.world.character_index {
            Some(index) => self.world.characters.get(index),
            None => todo!(),
        };

        let position = character.map_or(camera.position, |character| {
            character.transform.position + Vec3::new(0.0, CHARACTER_HEIGHT / 2.0, 0.0)
        });

        camera.position = position
    }

    fn update_parts(&mut self, dt: f32) {
        let mut to_remove = Vec::new();

        for (index, part) in self.world.parts.iter_mut() {
            let rigid_body = self
                .physics_context
                .rigid_body_set
                .get(part.rigid_body.rigid_body_handle)
                .unwrap();

            let pose = rigid_body.position();

            if pose.translation.y < -500.0 {
                to_remove.push(index);
                continue;
            }

            let pos_interpolated = rigid_body.predict_position_using_velocity(dt);

            part.transform.position = pos_interpolated.translation;
            part.transform.orientation = pos_interpolated.rotation;
        }

        for index in to_remove {
            let Some(part) = self.world.parts.remove(index) else {
                continue;
            };

            part.destroy(&mut self.physics_context);
        }
    }

    pub fn update(&mut self, window: &Window) {
        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32();
        let _elapsed = (now - self.start_time).as_secs_f32();

        self.last_frame = now;
        self.accumulator += dt;
        self.draw_accumulator += dt;

        while self.accumulator > STEP_HZ {
            self.update_fixed();
            self.accumulator -= STEP_HZ;
        }

        self.update_character_movement(self.accumulator);
        self.update_parts(self.accumulator);
        self.update_camera(dt, window);

        self.scene.ui.clear();

        let character = match self.world.character_index {
            Some(index) => self.world.characters.get(index),
            None => todo!(),
        };

        if let Some(character) = character {
            let horizontal_speed = (character.controller.velocity * Vec3::new(1.0, 0.0, 1.0))
                .length()
                .floor();

            self.speed_textbox
                .set_text(format!("speed: {}", horizontal_speed).into());

            {
                let mut font_manager = self.font_manager.lock().unwrap();
                self.speed_textbox.calculate_layout(&mut font_manager);
            }

            self.scene.push_ui_text(UiText::from_text_box(
                &self.speed_textbox,
                Vec2::new(0.0, 100.0),
            ));
        }

        self.input_state.clear();
    }

    fn draw(&mut self) {
        self.scene.meshes.clear();

        for (_i, part) in self.world.parts.iter() {
            self.scene.meshes.push(MeshNode {
                position: part.transform.position,
                orientation: part.transform.orientation,
                size: part.transform.size,
                color: part.mesh.color,
                opacity: part.mesh.opacity,
                mesh_id: part.mesh.mesh_id,
                material_id: part.mesh.material,
            });
        }

        for (_i, character) in self.world.characters.iter() {
            self.scene.meshes.push(MeshNode {
                position: character.transform.position,
                orientation: character.transform.orientation,
                size: character.transform.size,
                color: character.mesh.color,
                opacity: character.mesh.opacity,
                mesh_id: character.mesh.mesh_id,
                material_id: character.mesh.material,
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
            DeviceEvent::Key(key_event) => {
                self.input_state.raw_key_input(key_event);
            }
            _ => {}
        }
    }

    pub fn handle_window_event(&mut self, window: &Window, event: &WindowEvent) {
        let mut should_draw = false;

        match event {
            WindowEvent::KeyboardInput {
                device_id: _,
                event: _,
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
            WindowEvent::RedrawRequested => {
                should_draw = true;
                window.request_redraw();
            }
            _ => {}
        }

        self.update(window);

        if should_draw {
            // if self.draw_accumulator >= FRAMERATE_LIMIT_HZ {
            self.draw();
            // self.draw_accumulator = 0.0;
            // }
        }
    }
}
