use std::borrow::Cow;

use glam::Vec4;

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
            font_height: 14,
        }
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
}
