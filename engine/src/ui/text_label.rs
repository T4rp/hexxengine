use std::borrow::Cow;

use glam::{Vec2, Vec4};

use crate::{
    text::{FontHandle, TextBox},
    ui::{Element, UiDim, UiElement},
};

pub struct TextLabel {
    pub(super) text_box: TextBox,
    pub(super) font: Option<FontHandle>,
    pub(super) font_height: u32,

    pub text: Cow<'static, str>,
    pub color: Vec4,
    pub position: UiDim,
    pub size: UiDim,
    pub anchor: Vec2,
}

impl TextLabel {
    pub fn new(font: FontHandle) -> Self {
        Self {
            text_box: TextBox::new(font),
            text: "".into(),
            font: Some(font),
            color: Vec4::ZERO,
            position: UiDim::default(),
            size: UiDim::default(),
            anchor: Vec2::ZERO,
            font_height: 14,
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
}
