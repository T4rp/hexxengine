use std::{
    fs,
    sync::{Arc, Mutex},
    time::Instant,
};

use glam::{EulerRot, Quat, vec3};
use winit::window::Window;

use crate::{
    assets::{ASSET_PATH, get_first_gltf_mesh},
    game::{CUBE_MESH_ID, NOTOSANS_FONT_ID, SPHERE_MESH_ID, UNIFONT_FONT_ID},
    input::InputHandler,
    physics::context::PhysicsContext,
    renderer::renderer::{FALLBACK_SKYBOX_INDEX, VulkanContext},
    scene::{Camera, Lighting, RenderScene},
    text::FontManager,
    ui::UiTree,
};

pub struct GameContext {
    pub window: Window,
    pub font_manager: Arc<Mutex<FontManager>>,
    pub vk_ctx: VulkanContext,
    pub physics_context: PhysicsContext,
    pub input_state: InputHandler,
    pub render_scene: RenderScene,
    pub ui_tree: UiTree,
    pub start_time: Instant,
    pub last_frame: Instant,
    pub accumulator: f32,
}

impl GameContext {
    pub(crate) fn new(window: Window) -> Self {
        let mut font_manager = FontManager::new();
        let font_manager = Arc::new(Mutex::new(font_manager));

        let unifont_font_data = fs::read(format!("{}/unifont-17.0.03.otf", ASSET_PATH)).unwrap();

        let notosans_font_data = fs::read(format!(
            "{}//Noto_Sans/NotoSans-VariableFont_wdth,wght.ttf",
            ASSET_PATH
        ))
        .unwrap();

        {
            let mut font_manager = font_manager.lock().unwrap();

            font_manager
                .load_font_with_index(UNIFONT_FONT_ID, &unifont_font_data)
                .unwrap();

            font_manager
                .load_font_with_index(NOTOSANS_FONT_ID, &notosans_font_data)
                .unwrap();
        }

        let mut vk_ctx = VulkanContext::new(&window, font_manager.clone());
        let input_state = InputHandler::new();

        let cube_mesh = get_first_gltf_mesh(format!("{}/cube.gltf", ASSET_PATH).as_str());
        let sphere_mesh = get_first_gltf_mesh(format!("{}/sphere.gltf", ASSET_PATH).as_str());

        vk_ctx.load_mesh_with_index(CUBE_MESH_ID, &cube_mesh.vertices, &cube_mesh.indices);
        vk_ctx.load_mesh_with_index(SPHERE_MESH_ID, &sphere_mesh.vertices, &sphere_mesh.indices);

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

        let mut physics_context = PhysicsContext::new();

        let mut ui_tree = UiTree::new();

        let start_time = Instant::now();

        Self {
            window,
            font_manager,
            vk_ctx,
            physics_context,
            input_state,
            render_scene,
            ui_tree,
            start_time,
            last_frame: start_time,
            accumulator: 0.0,
        }
    }
}
