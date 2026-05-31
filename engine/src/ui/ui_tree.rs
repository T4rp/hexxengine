use std::borrow::Cow;

use glam::{Vec2, Vec4};
use thunderdome::{Arena, Index};

use crate::text::{FontHandle, TextBox};

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

pub struct Frame {
    pub color: Vec4,
    pub position: UiDim,
    pub size: UiDim,
}

pub struct TextLabel {
    text_box: TextBox,
    text: Cow<'static, str>,
    font: Option<FontHandle>,

    pub color: Vec4,
    pub position: UiDim,
    pub size: UiDim,
}

pub enum UiElement {
    Frame(Frame),
    TextLabel(TextLabel),
}

pub struct UiNode {
    pub world_position: Vec2,
    pub world_scale: Vec2,
    pub parent: Option<Index>,
    pub first_child: Option<Index>,
    pub next_sibling: Option<Index>,
    pub prev_sibling: Option<Index>,
    pub element: UiElement,
}

pub struct UiTree {
    elements: Arena<UiNode>,
    roots: Vec<Index>,
    root_size: Vec2,
}

impl UiTree {
    pub fn new() -> Self {
        Self {
            elements: Arena::new(),
            roots: Vec::new(),
            root_size: Vec2::ZERO,
        }
    }

    pub fn add_element() -> Index {
        todo!()
    }
}
