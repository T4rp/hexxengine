use hexxengine::{
    assets::{self, load_skybox},
    components::{MeshComponent, TransformComponent},
    entities::Part,
    game::{CUBE_MESH_ID, GameContext, GameHandler, NOTOSANS_FONT_HANDLE},
    glam::{Quat, Vec3, vec3},
    rapier3d::prelude::{RigidBodyType, ShapeType},
    renderer::render_scene::MeshNode,
    renderer::renderer::BASE_MATERIAL_INDEX,
    text::FontHandle,
    thunderdome::Arena,
    winit::{self, keyboard::KeyCode},
};

use crate::ui::{self, LevelEditorUi};

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
    font: FontHandle,
    world: World,
    level_editor: LevelEditorUi,
}

impl Game {
    fn update_camera(&mut self, game_ctx: &mut GameContext, dt: f32) {
        let camera = &mut game_ctx.render_scene.camera;

        let camera_forward = camera.orientation * Vec3::NEG_Z;
        let camera_right = camera.orientation * Vec3::X;

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

        if game_ctx.input_state.is_key_down(KeyCode::KeyA) {
            camera.position -= camera_right * dt * CAMERA_SPEED;
        }

        if game_ctx.input_state.is_key_down(KeyCode::KeyD) {
            camera.position += camera_right * dt * CAMERA_SPEED;
        }

        if game_ctx.input_state.is_key_down(KeyCode::KeyW) {
            camera.position += camera_forward * dt * CAMERA_SPEED;
        }

        if game_ctx.input_state.is_key_down(KeyCode::KeyS) {
            camera.position -= camera_forward * dt * CAMERA_SPEED;
        }
    }
}

impl GameHandler for Game {
    fn new(game_ctx: &mut GameContext) -> Self {
        let skybox = load_skybox(
            &mut game_ctx.vk_ctx,
            assets::get_asset_path("cloudy-skyboxes/Cubemap/Cubemap_Sky_04-512x512.png"),
        );

        game_ctx.render_scene.lighting.skybox_id = skybox;

        let mut world = World::new();

        let baseplate = Part::new(
            &mut game_ctx.physics_context,
            TransformComponent {
                position: vec3(0.0, -25.0, 0.0),
                orientation: Quat::IDENTITY,
                size: vec3(2048.0, 50.0, 2048.0),
            },
            MeshComponent {
                color: vec3(0.2, 0.2, 0.2),
                mesh_id: CUBE_MESH_ID,
                material: BASE_MATERIAL_INDEX,
                opacity: 1.0,
            },
            RigidBodyType::Fixed,
            ShapeType::Cuboid,
        );

        world.parts.insert(baseplate);

        // ui::init(&mut game_ctx.ui_tree, NOTOSANS_FONT_HANDLE);
        let level_editor = LevelEditorUi::new(&mut game_ctx.ui_tree);

        Game {
            font: NOTOSANS_FONT_HANDLE,
            world,
            level_editor,
        }
    }

    fn update(&mut self, game_ctx: &mut GameContext, dt: f32) {
        self.update_camera(game_ctx, dt);
        self.level_editor.update(game_ctx);
    }

    fn draw(&mut self, game_ctx: &mut GameContext) {
        game_ctx.render_scene.meshes.clear();

        for (_i, part) in self.world.parts.iter() {
            part.draw(&mut game_ctx.render_scene);
        }
    }
}
