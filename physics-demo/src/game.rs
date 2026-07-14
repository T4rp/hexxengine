use hexxengine::{
    assets::ASSET_PATH,
    game::{CUBE_MESH_ID, GameContext, GameHandler, NOTOSANS_FONT_HANDLE, SPHERE_MESH_ID},
    glam::{self, vec2},
    rand, rapier3d,
    renderer::renderer::BASE_MATERIAL_INDEX,
    scene::UiText,
    text::TextBox,
    thunderdome::{self, Index},
    winit,
};

use glam::{EulerRot, Quat, Vec3, vec3};
use rapier3d::prelude::RigidBodyType;

use rand::{Rng, SeedableRng, rngs::SmallRng};
use thunderdome::Arena;
use winit::keyboard::KeyCode;

use hexxengine::{
    assets::load_skybox,
    color::hsv_to_rgb,
    scene::{Camera, Lighting, MeshNode},
};

use crate::part::{Part, PartShape};

const CAMERA_SPEED: f32 = 100.0;

struct GameResources {
    skybox1: Index,
    skybox2: Index,
}

pub struct Game {
    rng: SmallRng,
    resources: GameResources,
    parts: Arena<Part>,
}

impl GameHandler for Game {
    fn new(game_ctx: &mut hexxengine::game::GameContext) -> Self {
        let skybox1_id = load_skybox(
            &mut game_ctx.vk_ctx,
            format!(
                "{}/cloudy-skyboxes/Cubemap/Cubemap_Sky_04-512x512.png",
                ASSET_PATH
            )
            .as_str(),
        );

        let skybox2_id = load_skybox(
            &mut game_ctx.vk_ctx,
            format!(
                "{}/cloudy-skyboxes/Cubemap/Cubemap_Sky_02-512x512.png",
                ASSET_PATH
            )
            .as_str(),
        );

        let resources = GameResources {
            skybox1: skybox1_id,
            skybox2: skybox2_id,
        };

        game_ctx.render_scene.camera = Camera::new(
            vec3(0.0, 100.0, 100.0),
            Quat::from_euler(EulerRot::ZXY, 0.0, f32::to_radians(-45.0), 0.0),
            90.0,
        );

        game_ctx.render_scene.lighting = Lighting {
            sun_direction: vec3(0.0, -1.0, -1.0).normalize(),
            sun_color: vec3(1.0, 0.95, 0.85),
            sun_power: 0.5,
            ambient_color: vec3(0.9, 0.95, 1.0) * 0.2,
            skybox_id: skybox1_id,
        };

        let mut rng = SmallRng::from_os_rng();

        let mut parts = Arena::new();

        parts.insert(Part::new_cube(
            &mut game_ctx.physics_context,
            RigidBodyType::Fixed,
            vec3(0.0, -25.0, 0.0),
            Quat::IDENTITY,
            vec3(2048.0, 50.0, 2048.0),
            vec3(0.8, 0.8, 0.8),
        ));

        for _ in 0..200 {
            let cuboid = Part::new_cube(
                &mut game_ctx.physics_context,
                RigidBodyType::Dynamic,
                vec3(
                    rng.random_range(-50.0..50.0),
                    rng.random_range(1.0..50.0),
                    rng.random_range(-50.0..50.0),
                ) * 5.0,
                Quat::from_euler(
                    EulerRot::XYZ,
                    rng.random::<f32>() * std::f32::consts::PI * 2.0,
                    rng.random::<f32>() * std::f32::consts::PI * 2.0,
                    rng.random::<f32>() * std::f32::consts::PI * 2.0,
                ),
                vec3(4.0, 4.0, 4.0) * rng.random_range(1.0..5.0),
                hsv_to_rgb(rng.random::<f32>() * 360.0, 0.8, 1.0),
            );

            parts.insert(cuboid);
        }

        let mut textbox = TextBox::from_text(
            NOTOSANS_FONT_HANDLE,
            16,
            "the quick brown fox doesnt not concern himself with subpixel rendering".into(),
        );

        {
            let mut font_manager = game_ctx.font_manager.lock().unwrap();
            textbox.calculate_layout(&mut font_manager);
        }

        game_ctx
            .render_scene
            .push_ui_text(UiText::from_text_box(&textbox, vec2(0.0, 16.0)));

        Self {
            rng,
            resources,
            parts,
        }
    }

    fn fixed_update(&mut self, game_ctx: &mut GameContext, _dt: f32) {
        game_ctx.physics_context.step();
    }

    fn update(&mut self, game_ctx: &mut GameContext, dt: f32) {
        self.move_camera(game_ctx, dt);
        self.handle_spawning_parts(game_ctx);
        self.clean_parts(game_ctx);

        let elapsed = (game_ctx.last_frame - game_ctx.start_time).as_secs_f32();

        let skybox_switch = ((elapsed / 10.0).floor() as i32) % 10;

        if skybox_switch % 2 == 0 {
            game_ctx.render_scene.lighting.skybox_id = self.resources.skybox1
        } else {
            game_ctx.render_scene.lighting.skybox_id = self.resources.skybox2
        }
    }

    fn draw(&mut self, game_ctx: &mut GameContext) {
        game_ctx.render_scene.meshes.clear();

        for (_i, part) in self.parts.iter() {
            match part.shape {
                PartShape::Cube(size) => {
                    game_ctx.render_scene.meshes.push(MeshNode {
                        position: part.position,
                        orientation: part.orientation,
                        size,
                        color: part.color,
                        opacity: 1.0,
                        mesh_id: CUBE_MESH_ID,
                        material_id: BASE_MATERIAL_INDEX,
                    });
                }
                PartShape::Sphere(radius) => {
                    game_ctx.render_scene.meshes.push(MeshNode {
                        position: part.position,
                        orientation: part.orientation,
                        size: Vec3::splat(radius * 2.0),
                        color: part.color,
                        opacity: 1.0,
                        mesh_id: SPHERE_MESH_ID,
                        material_id: BASE_MATERIAL_INDEX,
                    });
                }
            }
        }
    }
}

impl Game {
    fn move_camera(&mut self, game_ctx: &mut GameContext, dt: f32) {
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
            let sensitivity = 0.001;

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

    fn handle_spawning_parts(&mut self, game_ctx: &mut GameContext) {
        let camera_position = game_ctx.render_scene.camera.position;
        let camera_forward = game_ctx.render_scene.camera.orientation * Vec3::NEG_Z;

        if game_ctx.input_state.is_key_down(KeyCode::Space) {
            let rng = &mut self.rng;

            let part = if rng.random_bool(0.5) {
                Part::new_cube(
                    &mut game_ctx.physics_context,
                    RigidBodyType::Dynamic,
                    camera_position + camera_forward * 30.0,
                    Quat::from_euler(
                        EulerRot::XYZ,
                        rng.random::<f32>() * std::f32::consts::PI * 2.0,
                        rng.random::<f32>() * std::f32::consts::PI * 2.0,
                        rng.random::<f32>() * std::f32::consts::PI * 2.0,
                    ),
                    vec3(4.0, 4.0, 4.0) * rng.random_range(1.0..5.0),
                    hsv_to_rgb(rng.random::<f32>() * 360.0, 0.8, 1.0),
                )
            } else {
                Part::new_sphere(
                    &mut game_ctx.physics_context,
                    RigidBodyType::Dynamic,
                    camera_position + camera_forward * 30.0,
                    Quat::from_euler(
                        EulerRot::XYZ,
                        rng.random::<f32>() * std::f32::consts::PI * 2.0,
                        rng.random::<f32>() * std::f32::consts::PI * 2.0,
                        rng.random::<f32>() * std::f32::consts::PI * 2.0,
                    ),
                    rng.random_range(5.0..10.0),
                    hsv_to_rgb(rng.random::<f32>() * 360.0, 0.8, 1.0),
                )
            };

            let rigid_body = game_ctx
                .physics_context
                .rigid_body_set
                .get_mut(part.rigid_body_handle)
                .unwrap();
            rigid_body.set_linvel(camera_forward * 500.0, true);
            self.parts.insert(part);
        }
    }

    fn clean_parts(&mut self, game_ctx: &mut GameContext) {
        let mut to_remove = Vec::new();

        for (index, cube) in self.parts.iter_mut() {
            let rigid_body = game_ctx
                .physics_context
                .rigid_body_set
                .get(cube.rigid_body_handle)
                .unwrap();

            let pose = rigid_body.position();

            if pose.translation.y < -500.0 {
                to_remove.push(index);
            }

            cube.position = pose.translation;
            cube.orientation = pose.rotation;
        }

        for index in to_remove {
            let Some(cube) = self.parts.remove(index) else {
                continue;
            };

            cube.destroy(&mut game_ctx.physics_context);
        }
    }
}
