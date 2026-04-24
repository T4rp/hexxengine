use hexxengine::{
    glam::{Vec2, Vec3, ivec2, vec2},
    renderer::renderer::WHITE_TEXTURE_INDEX,
    scene::{RenderScene, UiDraw, UiFrame, UiText},
};

pub struct EditorUi {}

impl EditorUi {
    pub fn new() -> Self {
        EditorUi {}
    }

    pub fn render(&self, render_scene: &mut RenderScene) {
        render_scene.ui.push(UiDraw::Frame(UiFrame::new(
            vec2(0.0, 100.0 - 16.0),
            vec2(100.0, 18.0),
            WHITE_TEXTURE_INDEX,
        )));

        render_scene.ui.push(UiDraw::Text(UiText {
            position: ivec2(0, 100),
            anchor: Vec2::ZERO,
            font_height: 16,
            text: "balls".into(),
            color: Vec3::ONE,
        }));
    }
}
