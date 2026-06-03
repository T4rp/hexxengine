use glam::{Vec2, Vec4};

use crate::ui::{Element, UiDim, UiElement};

#[derive(Default)]
pub struct Frame {
    pub color: Vec4,
    pub anchor: Vec2,
    pub position: UiDim,
    pub size: UiDim,
}

impl UiElement for Frame {
    fn position(&self) -> UiDim {
        self.position
    }

    fn size(&self) -> UiDim {
        self.size
    }

    fn to_enum(self) -> Element {
        Element::Frame(self)
    }

    fn anchor(&self) -> Vec2 {
        self.anchor
    }
}
