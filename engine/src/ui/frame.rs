use glam::{Vec2, Vec4};

use crate::ui::{Element, UiDim, UiElement};

pub struct Frame {
    pub color: Vec4,
    pub anchor: Vec2,
    pub position: UiDim,
    pub visible: bool,
    pub size: UiDim,
}

impl Default for Frame {
    fn default() -> Self {
        Self {
            color: Default::default(),
            anchor: Default::default(),
            position: Default::default(),
            visible: true,
            size: Default::default(),
        }
    }
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

    fn visible(&self) -> bool {
        self.visible
    }
}
