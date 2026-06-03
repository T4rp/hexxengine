use glam::Vec2;

use crate::ui::{Frame, TextLabel};

#[derive(Clone, Copy, Default, Debug)]
pub struct UiDim {
    pub scale: Vec2,
    pub offset: Vec2,
}

impl UiDim {
    pub fn new(scale_x: f32, scale_y: f32, offset_x: f32, offset_y: f32) -> Self {
        Self {
            scale: Vec2::new(scale_x, scale_y),
            offset: Vec2::new(offset_x, offset_y),
        }
    }
}

pub trait UiElement {
    fn position(&self) -> UiDim;
    fn size(&self) -> UiDim;
    fn to_enum(self) -> Element;
}

pub enum Element {
    Frame(Frame),
    TextLabel(TextLabel),
}

impl Element {
    pub fn position(&self) -> UiDim {
        match self {
            Element::Frame(frame) => frame.position,
            Element::TextLabel(text_label) => text_label.position,
        }
    }

    pub fn size(&self) -> UiDim {
        match self {
            Element::Frame(frame) => frame.size,
            Element::TextLabel(text_label) => text_label.size,
        }
    }
}
