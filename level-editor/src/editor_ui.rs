use std::borrow::Cow;

use hexxengine::font_manager::FontHandle;
use hexxengine::glam::{Vec4, vec2};
use hexxengine::renderer::renderer::WHITE_TEXTURE_INDEX;
use hexxengine::scene::{UiDraw, UiFrame, UiText};
use hexxengine::{
    glam::{Vec2, Vec3},
    thunderdome::{Arena, Index},
};

pub struct UiNode {
    pub parent: Option<Index>,
    pub children: Vec<Index>,
    pub world_position: Vec2,
    pub world_size: Vec2,
    pub element: UiElement,
}

#[derive(Default)]
pub struct UiRect {
    pub color: Vec3,
    pub position: Vec4,
    pub size: Vec4,
}

impl UiRect {
    pub fn to_elem(self) -> UiElement {
        UiElement::Rect(self)
    }
}

pub struct UiTextBox {
    pub font: FontHandle,
    pub color: Vec3,
    pub position: Vec4,
    pub size: Vec4,
    pub font_size: u32,
    pub text: Cow<'static, str>,
}

impl UiTextBox {
    pub fn to_elem(self) -> UiElement {
        UiElement::Text(self)
    }
}

pub enum UiElement {
    Rect(UiRect),
    Text(UiTextBox),
}

impl UiElement {
    pub fn position(&self) -> Vec4 {
        match self {
            UiElement::Rect(ui_rect) => ui_rect.position,
            UiElement::Text(ui_text) => ui_text.position,
        }
    }

    pub fn size(&self) -> Vec4 {
        match self {
            UiElement::Rect(ui_rect) => ui_rect.size,
            UiElement::Text(ui_text) => ui_text.size,
        }
    }
}

pub struct UiContext {
    pub elements: Arena<UiNode>,
    pub root: Vec<Index>,
    pub root_size: Vec2,
}

impl UiContext {
    pub fn new() -> Self {
        Self {
            elements: Arena::new(),
            root: Vec::new(),
            root_size: Vec2::ZERO,
        }
    }

    pub fn clear(&mut self) {
        self.elements.clear();
        self.root.clear();
    }

    pub fn parent(&mut self, child: Index, parent: Index) {
        let parent_element = self.elements.get_mut(parent).unwrap();
        parent_element.children.push(child);

        let child_element = self.elements.get_mut(child).unwrap();
        child_element.parent = Some(parent);
    }

    pub fn root(&mut self, root_element: Index) {
        self.root.push(root_element);
    }

    pub fn set_root_size(&mut self, size: Vec2) {
        self.root_size = size;
    }

    pub fn new_elem(&mut self, element: UiElement) -> Index {
        let element = UiNode {
            parent: None,
            children: Vec::new(),
            world_position: Vec2::ZERO,
            world_size: Vec2::ZERO,
            element,
        };

        let index = self.elements.insert(element);

        index
    }

    pub fn build_draws(&mut self, ui_draws: &mut Vec<UiDraw>) {
        let mut elements = self.root.clone();

        while elements.len() > 0 {
            let elem_i = elements.pop().unwrap();
            let elem = self.elements.get(elem_i).unwrap();

            let elem_position = elem.element.position();
            let elem_size = elem.element.size();

            let parent_elem = elem.parent.map_or(None, |i| self.elements.get_mut(i));

            let (parent_pos, parent_size) = if let Some(parent) = parent_elem {
                (parent.world_position, parent.world_size)
            } else {
                (Vec2::ZERO, self.root_size)
            };

            let position = parent_pos
                + parent_size * vec2(elem_position.x, elem_position.z)
                + vec2(elem_position.y, elem_position.w);

            let size =
                parent_size * vec2(elem_size.x, elem_size.z) + vec2(elem_size.y, elem_size.w);

            let elem = self.elements.get_mut(elem_i).unwrap();
            elem.world_position = position;
            elem.world_size = size;

            for child in elem.children.iter() {
                elements.push(*child);
            }

            match &elem.element {
                UiElement::Rect(ui_rect) => {
                    let color = ui_rect.color;
                    ui_draws.push(UiDraw::Frame(UiFrame {
                        position: elem.world_position,
                        anchor: Vec2::ZERO,
                        size: elem.world_size,
                        color: Vec4::new(color.x, color.y, color.z, 1.0),
                        texture_id: WHITE_TEXTURE_INDEX,
                        uvs: Default::default(),
                    }));
                }
                UiElement::Text(ui_text) => {
                    ui_draws.push(UiDraw::Text(UiText {
                        font: ui_text.font,
                        position: elem.world_position,
                        anchor: Vec2::ZERO,
                        font_height: ui_text.font_size,
                        text: ui_text.text.clone(),
                        color: ui_text.color,
                    }));
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use hexxengine::glam::{vec2, vec3, vec4};

    use crate::editor_ui::{UiContext, UiRect};

    #[test]
    fn build_tree() {
        let mut ctx = UiContext::new();

        let pane = ctx.new_elem(
            UiRect {
                position: vec4(0.0, 0.0, 0.0, 0.0),
                size: vec4(0.0, 100.0, 0.0, 50.0),
                color: vec3(1.0, 1.0, 1.0),
                ..Default::default()
            }
            .to_elem(),
        );
        ctx.root(pane);

        let inner_pane = ctx.new_elem(
            UiRect {
                size: vec4(0.0, 100.0, 0.0, 50.0),
                ..Default::default()
            }
            .to_elem(),
        );
        ctx.parent(inner_pane, pane);

        let mut ui_draws = Vec::new();
        ctx.build_draws(&mut ui_draws);
    }
}
