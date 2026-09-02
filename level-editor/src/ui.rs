use hexxengine::{
    glam::{Vec2, Vec4, vec3, vec4},
    renderer::render_scene::RenderScene,
    text::{
        FontHandle, FontManager,
        textbox::{HorizontalJustification, VerticalJustification},
    },
    thunderdome::Index,
    ui::{Frame, TextLabel, UiDim, UiEventType, UiTree},
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
        ..Default::default()
    });

    ctx.root(pane);

    for i in 0..10 {
        let mut text_label = TextLabel::new(font);
        text_label.color = COLOR_TEXT;
        text_label.background_color = if i % 2 == 0 { COLOR_FG1 } else { COLOR_FG2 };
        text_label.size = UiDim::new(1.0, 0.0, 0.0, 25.0);
        text_label.position = UiDim::new(0.0, 0.0, 0.0, 25.0 * i as f32);
        text_label.set_horizontal_justification(HorizontalJustification::Center);
        text_label.set_vertical_justification(VerticalJustification::Center);
        text_label.set_text(format!("hello world {}", i.to_string()).into());

        let text_label = ctx.add_element(text_label);
        ctx.parent(pane, text_label);
    }
}

pub struct LevelEditorUi {
    elements: Vec<Index>,
}

impl LevelEditorUi {
    pub fn new(ctx: &mut UiTree) -> Self {
        let dock = ctx.add_element(Frame {
            color: COLOR_BG,
            anchor: Vec2::ZERO,
            position: UiDim::new(0.0, 0.0, 0.0, 0.0),
            visible: true,
            size: UiDim::new(1.0, 0.0, 0.0, 100.0),
        });

        ctx.root(dock);

        Self { elements: vec![] }
    }

    pub fn update(&mut self) {}
}

pub struct CookieClicker {
    elements: Vec<Index>,
    cookie_count_label: Index,
    pane: Index,
    cookies: u32,
}

impl CookieClicker {
    pub fn new(ctx: &mut UiTree, font: FontHandle) -> CookieClicker {
        let pane = ctx.add_element(Frame {
            color: COLOR_BG,
            anchor: Vec2::new(0.5, 0.5),
            position: UiDim::new(0.5, 0.5, 0.0, 0.0),
            size: UiDim::new(0.0, 0.0, 400.0, 300.0),
            ..Default::default()
        });

        ctx.enable_events(pane);
        ctx.root(pane);

        let mut text_label = TextLabel::new(font);
        text_label.color = COLOR_TEXT;
        text_label.background_color = COLOR_BG + Vec4::new(0.01, 0.01, 0.01, 0.0);
        text_label.size = UiDim::new(1.0, 0.0, 0.0, 20.0);
        text_label.anchor = Vec2::new(0.0, 0.0);
        text_label.position = UiDim::new(0.0, 0.0, 0.0, 0.0);
        text_label.set_horizontal_justification(HorizontalJustification::Center);
        text_label.set_vertical_justification(VerticalJustification::Center);
        text_label.set_text(format!("{} cookies", 0).into());

        let text_label = ctx.add_element(text_label);
        ctx.parent(pane, text_label);

        Self {
            elements: vec![pane, text_label],
            pane: pane,
            cookie_count_label: text_label,
            cookies: 0,
        }
    }

    pub fn update(&mut self, ctx: &mut UiTree) {
        let pane_node = ctx.get_element(self.pane).unwrap();

        for event in pane_node.events.as_ref().unwrap() {
            if event.event_type != UiEventType::Pressed {
                continue;
            }

            self.cookies += 1;
        }

        ctx.clear_events(self.pane);

        let text_label = ctx
            .get_element_mut(self.cookie_count_label)
            .unwrap()
            .element
            .text_label_mut()
            .unwrap();

        text_label.set_text(format!("{} cookies", self.cookies).into());
    }
}
