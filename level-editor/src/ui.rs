use hexxengine::{
    glam::{vec2, vec3},
    scene::RenderScene,
};

use crate::editor_ui::{UiContext, UiRect};

pub fn render(scene: &mut RenderScene, ctx: &mut UiContext) {
    let pane = ctx.new_elem(
        UiRect {
            position: vec2(100.0, 100.0),
            size: vec2(100.0, 50.0),
            color: vec3(1.0, 1.0, 1.0),
            ..Default::default()
        }
        .to_elem(),
    );
    ctx.root(pane);

    let inner_pane = ctx.new_elem(
        UiRect {
            size: vec2(20.0, 20.0),
            ..Default::default()
        }
        .to_elem(),
    );
    ctx.parent(inner_pane, pane);

    ctx.build_draws(&mut scene.ui);
}
