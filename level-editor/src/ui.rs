use hexxengine::{
    game::{GameContext, UNIFONT_FONT_HANDLE},
    glam::{Vec2, Vec4},
    text::textbox::{HorizontalJustification, VerticalJustification},
    thunderdome::Index,
    ui::{Frame, TextLabel, UiDim, UiEventType, UiTree},
};

const COLOR_BG: Vec4 = Vec4::new(0.01, 0.01, 0.01, 1.0);
const COLOR_BG2: Vec4 = Vec4::new(0.05, 0.05, 0.05, 1.0);
const COLOR_TEXT: Vec4 = Vec4::new(0.9, 0.9, 0.9, 1.0);

pub struct LevelEditorUi {
    elements: Vec<Index>,
    add_part_button: Index,
}

impl LevelEditorUi {
    pub fn new(ctx: &mut UiTree) -> Self {
        let dock = ctx.add_element(Frame {
            color: COLOR_BG,
            anchor: Vec2::ZERO,
            position: UiDim::new(0.0, 0.0, 0.0, 0.0),
            visible: true,
            size: UiDim::new(0.0, 0.0, 500.0, 100.0),
        });

        ctx.root(dock);

        let mut part_button_label = TextLabel::new(UNIFONT_FONT_HANDLE);
        part_button_label.set_vertical_justification(VerticalJustification::Center);
        part_button_label.set_horizontal_justification(HorizontalJustification::Left);
        part_button_label.size = UiDim::new(0.0, 0.0, 100.0, 30.0);
        part_button_label.color = COLOR_TEXT;
        part_button_label.background_color = COLOR_BG2;
        part_button_label.set_text("Add part".into());
        part_button_label.set_font_height(18);

        let add_part_button = ctx.add_element(part_button_label);

        ctx.parent(dock, add_part_button);
        ctx.enable_events(add_part_button);

        Self {
            elements: vec![dock, add_part_button],
            add_part_button,
        }
    }

    pub fn update(&mut self, game_ctx: &mut GameContext) {
        let button = game_ctx.ui_tree.get_element(self.add_part_button).unwrap();

        for event in button.events.as_ref().unwrap() {
            if event.event_type != UiEventType::Pressed {
                continue;
            }

            println!("Add part button pressed");
        }

        game_ctx.ui_tree.clear_events(self.add_part_button);
    }
}
