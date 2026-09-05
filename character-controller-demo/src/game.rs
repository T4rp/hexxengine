use std::{
    fs,
    sync::{Arc, Mutex},
    time::Instant,
};

use hexxengine::{
    assets::{self, get_first_gltf_mesh, load_skybox},
    color::hsv_to_rgb,
    components::{MeshComponent, RigidBodyComponent, TransformComponent},
    entities::{Camera, Part},
    game::{CUBE_MESH_ID, GameContext, GameHandler, UNIFONT_FONT_HANDLE},
    glam::{self, Vec2, Vec4},
    input::InputHandler,
    physics::context::PhysicsContext,
    rand,
    rapier3d::{self, prelude::ShapeType},
    renderer::{
        render_scene::{Lighting, MeshNode, RenderScene, UiText},
        renderer::{BASE_MATERIAL_INDEX, VulkanContext},
    },
    text::{FontHandle, FontManager, TextBox, textbox::HorizontalJustification},
    thunderdome::Index,
    ui::{TextLabel, UiDim},
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
    skybox1: Index,
}

pub struct Game {
    rng: SmallRng,
    resources: GameResources,
    world: World,
    speed_text: Index,
}

impl Game {
    fn update_camera(&mut self, game_ctx: &mut GameContext, dt: f32) {
        let camera = &mut game_ctx.render_scene.camera;

        if game_ctx.input_state.right_mouse_down {
            let _ = game_ctx
                .window
                .set_cursor_grab(winit::window::CursorGrabMode::Confined)
                .or_else(|_| {
                    game_ctx
                        .window
                        .set_cursor_grab(winit::window::CursorGrabMode::Locked)
                });
            game_ctx.window.set_cursor_visible(false);
        } else {
            let _ = game_ctx
                .window
                .set_cursor_grab(winit::window::CursorGrabMode::None);
            game_ctx.window.set_cursor_visible(true);
        }

        let mouse_delta = game_ctx.input_state.mouse_delta;

        if mouse_delta.z == 0.0 && game_ctx.input_state.right_mouse_down {
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

    fn update_parts(&mut self, game_ctx: &mut GameContext, dt: f32) {
        let mut to_remove = Vec::new();

        for (index, part) in self.world.parts.iter_mut() {
            part.update(&mut game_ctx.physics_context, dt);

            if part.transform.position.y < -500.0 {
                to_remove.push(index);
                continue;
            }
        }

        for index in to_remove {
            let Some(part) = self.world.parts.remove(index) else {
                continue;
            };

            part.destroy(&mut game_ctx.physics_context);
        }
    }

    fn update_character_movement(&mut self, game_ctx: &mut GameContext, dt: f32) {
        let mut move_dir = Vec3::ZERO;

        if game_ctx.input_state.is_key_down(KeyCode::KeyA) {
            move_dir += Vec3::new(-1.0, 0.0, 0.0)
        }

        if game_ctx.input_state.is_key_down(KeyCode::KeyD) {
            move_dir += Vec3::new(1.0, 0.0, 0.0)
        }

        if game_ctx.input_state.is_key_down(KeyCode::KeyW) {
            move_dir += Vec3::new(0.0, 0.0, -1.0)
        }

        if game_ctx.input_state.is_key_down(KeyCode::KeyS) {
            move_dir += Vec3::new(0.0, 0.0, 1.0)
        }

        let character = self
            .world
            .characters
            .get_mut(self.world.character_index.unwrap())
            .unwrap();

        if game_ctx.input_state.is_key_down(KeyCode::Space) {
            character.controller.jump = true;
        }

        character
            .controller
            .update_position(dt, &mut character.transform);

        let mut world_move = game_ctx.render_scene.camera.orientation * move_dir;
        world_move.y = 0.0;

        world_move = world_move.normalize_or_zero();

        character.controller.move_dir = world_move;
    }
}

impl GameHandler for Game {
    fn new(game_ctx: &mut GameContext) -> Self {
        let skybox1_id = load_skybox(
            &mut game_ctx.vk_ctx,
            assets::get_asset_path("cloudy-skyboxes/Cubemap/Cubemap_Sky_04-512x512.png")
                .to_str()
                .unwrap(),
        );

        let resources = GameResources {
            skybox1: skybox1_id,
        };

        game_ctx.render_scene.camera = Camera::new(
            vec3(0.0, 100.0, 100.0),
            Quat::from_euler(EulerRot::ZXY, 0.0, f32::to_radians(-45.0), 0.0),
            120.0,
        );

        game_ctx.render_scene.lighting = Lighting {
            sun_direction: vec3(0.0, -1.0, -1.0).normalize(),
            sun_color: vec3(1.0, 0.95, 0.85),
            sun_power: 0.5,
            ambient_color: vec3(0.9, 0.95, 1.0) * 0.2,
            skybox_id: skybox1_id,
        };

        let mut rng = SmallRng::from_os_rng();

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
                mesh_id: CUBE_MESH_ID,
                material: BASE_MATERIAL_INDEX,
                opacity: 1.0,
            },
            rigid_body: RigidBodyComponent::new(
                &mut game_ctx.physics_context,
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
                    mesh_id: CUBE_MESH_ID,
                    material: BASE_MATERIAL_INDEX,
                    opacity: 1.0,
                },
                rigid_body: RigidBodyComponent::new(
                    &mut game_ctx.physics_context,
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
                mesh_id: CUBE_MESH_ID,
                material: BASE_MATERIAL_INDEX,
                opacity: 1.0,
            },
            controller: CharacterControllerComponent::new(
                &mut game_ctx.physics_context,
                &character_transform,
            ),
        };

        let character_handle = world.characters.insert(character);
        world.character_index = Some(character_handle);

        let mut speed_textbox = TextBox::new(UNIFONT_FONT_HANDLE);
        speed_textbox.horizontal_justification = HorizontalJustification::Left;
        speed_textbox.font_height = 32;
        speed_textbox.size = Vec2::new(50.0, 10.0);

        let mut text_label = TextLabel::new(UNIFONT_FONT_HANDLE);
        text_label.position = UiDim::new(0.0, 0.0, 0.0, 20.0);
        text_label.size = UiDim::new(0.0, 0.0, 100.0, 18.0);
        text_label.color = Vec4::new(0.0, 0.0, 0.0, 1.0);
        text_label.set_font_height(30);
        text_label.set_horizontal_justification(HorizontalJustification::Left);

        let speed_text = game_ctx.ui_tree.add_element(text_label);
        game_ctx.ui_tree.root(speed_text);

        Self {
            rng,
            resources,
            world,
            speed_text,
        }
    }

    fn fixed_update(&mut self, game_ctx: &mut GameContext, dt: f32) {
        let character = self
            .world
            .characters
            .get_mut(self.world.character_index.unwrap())
            .unwrap();

        character
            .controller
            .move_dir(&mut game_ctx.physics_context, dt);

        game_ctx.physics_context.step();
    }

    fn update(&mut self, game_ctx: &mut GameContext, dt: f32) {
        self.update_character_movement(game_ctx, dt);
        self.update_parts(game_ctx, dt);
        self.update_camera(game_ctx, dt);

        game_ctx.render_scene.ui.clear();

        let character = match self.world.character_index {
            Some(index) => self.world.characters.get(index),
            None => todo!(),
        };

        if let Some(character) = character {
            let horizontal_speed = (character.controller.velocity * Vec3::new(1.0, 0.0, 1.0))
                .length()
                .floor();

            let elem = game_ctx.ui_tree.get_element_mut(self.speed_text).unwrap();

            elem.element
                .text_label_mut()
                .unwrap()
                .set_text(format!("speed: {}", horizontal_speed).into());
        }

        game_ctx.input_state.clear();
    }

    fn draw(&mut self, game_ctx: &mut GameContext) {
        game_ctx.render_scene.meshes.clear();

        for (_i, part) in self.world.parts.iter() {
            part.draw(&mut game_ctx.render_scene);
        }

        for (_i, character) in self.world.characters.iter() {
            game_ctx.render_scene.meshes.push(MeshNode {
                position: character.transform.position,
                orientation: character.transform.orientation,
                size: character.transform.size,
                color: character.mesh.color,
                opacity: character.mesh.opacity,
                mesh_id: character.mesh.mesh_id,
                material_id: character.mesh.material,
            });
        }
    }
}
