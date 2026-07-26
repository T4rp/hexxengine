use std::borrow::Cow;

use glam::{Vec2, Vec4};

use crate::{
    text::{
        FontHandle, TextBox,
        textbox::{HorizontalJustification, VerticalJustification},
    },
    ui::{Element, UiDim, UiElement},
};

pub struct TextLabel {
    pub(super) text_box: TextBox,
    pub(super) font: Option<FontHandle>,
    pub(super) font_height: u32,
    pub(super) horizontal_justification: HorizontalJustification,
    pub(super) vertical_justification: VerticalJustification,

    pub text: Cow<'static, str>,
    pub color: Vec4,
    pub background_color: Vec4,
    pub position: UiDim,
    pub size: UiDim,
    pub anchor: Vec2,
    pub visible: bool,
}

impl TextLabel {
    pub fn new(font: FontHandle) -> Self {
        let mut text_box = TextBox::new(font);
        text_box.set_horizontal_justification(HorizontalJustification::Center);
        text_box.set_vertical_justification(VerticalJustification::Center);
        text_box.set_font_height(14);

        Self {
            text_box: text_box,
            text: "".into(),
            font: Some(font),
            color: Vec4::ZERO,
            position: UiDim::default(),
            size: UiDim::default(),
            anchor: Vec2::ZERO,
            font_height: 14,
            horizontal_justification: HorizontalJustification::Center,
            vertical_justification: VerticalJustification::Center,
            visible: true,
            background_color: Vec4::ZERO,
        }
    }

    pub fn set_font_height(&mut self, font_height: u32) {
        self.font_height = font_height;
        self.text_box.set_font_height(font_height);
    }

    pub fn set_text(&mut self, text: Cow<'static, str>) {
        self.text = text.clone();
        self.text_box.set_text(text.clone());
    }

    pub fn set_font(&mut self, font: FontHandle) {
        self.font = Some(font);
        self.text_box.set_font(font);
    }

    pub fn set_horizontal_justification(
        &mut self,
        horizontal_justification: HorizontalJustification,
    ) {
        self.horizontal_justification = horizontal_justification;
        self.text_box
            .set_horizontal_justification(horizontal_justification);
    }

    pub fn set_vertical_justification(&mut self, vertical_justification: VerticalJustification) {
        self.vertical_justification = vertical_justification;
        self.text_box
            .set_vertical_justification(vertical_justification);
    }
}

impl UiElement for TextLabel {
    fn position(&self) -> UiDim {
        self.position
    }

    fn size(&self) -> UiDim {
        self.size
    }

    fn to_enum(self) -> Element {
        Element::TextLabel(self)
    }

    fn anchor(&self) -> Vec2 {
        self.anchor
    }

    fn visible(&self) -> bool {
        self.visible
    }
}
