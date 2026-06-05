use hexxengine::{
    glam::{Vec2, Vec4, vec3, vec4},
    scene::RenderScene,
    text::{
        FontHandle, FontManager,
        textbox::{HorizontalJustification, VerticalJustification},
    },
    ui::{Frame, TextLabel, UiDim, UiTree},
};

const COLOR_BG: Vec4 = Vec4::new(0.01, 0.01, 0.01, 1.0);
const COLOR_TEXT: Vec4 = Vec4::new(0.9, 0.9, 0.9, 1.0);
const COLOR_FG1: Vec4 = Vec4::new(0.05, 0.05, 0.05, 1.0);
const COLOR_FG2: Vec4 = Vec4::new(0.06, 0.06, 0.06, 1.0);

pub fn init(ctx: &mut UiTree, font: FontHandle) {
    let pane = ctx.add_element(Frame {
        color: COLOR_BG,
        anchor: Vec2::new(1.0, 0.0),
        position: UiDim::new(1.0, 0.0, 0.0, 0.0),
        size: UiDim::new(0.0, 1.0, 200.0, 0.0),
    });

    ctx.root(pane);

    for i in 0..10 {
        let inner_pane = ctx.add_element(Frame {
            position: UiDim::new(0.0, 0.0, 0.0, 25.0 * i as f32),
            size: UiDim::new(1.0, 0.0, 0.0, 25.0),
            color: if i % 2 == 0 { COLOR_FG1 } else { COLOR_FG2 },
            ..Default::default()
        });

        ctx.parent(pane, inner_pane);

        let mut text_label = TextLabel::new(font);
        text_label.color = COLOR_TEXT;
        text_label.size = UiDim::new(1.0, 1.0, 0.0, 0.0);
        text_label.set_horizontal_justification(HorizontalJustification::Center);
        text_label.set_vertical_justification(VerticalJustification::Center);
        text_label.set_text(format!("hello world {}", i.to_string()).into());

        let text_label = ctx.add_element(text_label);
        ctx.parent(inner_pane, text_label);
    }
}
