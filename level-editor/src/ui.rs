use hexxengine::{
    glam::{Vec2, Vec4, vec2, vec3, vec4},
    scene::RenderScene,
    text::FontHandle,
};

use crate::editor_ui::{UiContext, UiRect, UiTextBox};

pub fn render(scene: &mut RenderScene, ctx: &mut UiContext, font: FontHandle) {
    let pane = ctx.new_elem(
        UiRect {
            position: vec4(0.0, 100.0, 0.0, 100.0),
            size: vec4(0.0, 100.0, 0.5, 50.0),
            color: vec3(1.0, 1.0, 1.0),
            ..Default::default()
        }
        .to_elem(),
    );
    ctx.root(pane);

    let inner_pane = ctx.new_elem(
        UiRect {
            size: vec4(0.0, 20.0, 0.0, 20.0),
            ..Default::default()
        }
        .to_elem(),
    );
    ctx.parent(inner_pane, pane);

    let label = ctx.new_elem(
        UiTextBox {
            font: font,
            color: vec3(1.0, 1.0, 1.0),
            position: Vec4::ZERO,
            font_size: 18,
            text: "label".into(),
            size: vec4(0.0, 1.0, 0.0, 1.0),
        }
        .to_elem(),
    );
    ctx.parent(label, inner_pane);

    ctx.build_draws(&mut scene.ui);
}
