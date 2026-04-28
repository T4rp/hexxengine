use hexxengine::glam::{Vec4, vec2};
use hexxengine::renderer::renderer::WHITE_TEXTURE_INDEX;
use hexxengine::scene::{UiDraw, UiFrame};
use hexxengine::{
    glam::{Vec2, Vec3},
    thunderdome::{Arena, Index},
};

pub struct UiNode {
    pub parent: Option<Index>,
    pub children: Vec<Index>,
    pub world_position: Vec2,
    pub element: UiElement,
}

#[derive(Default)]
pub struct UiRect {
    pub color: Vec3,
    pub position: Vec2,
    pub size: Vec2,
}

impl UiRect {
    pub fn to_elem(self) -> UiElement {
        UiElement::Rect(self)
    }
}

pub enum UiElement {
    Rect(UiRect),
}

impl UiElement {
    pub fn position(&self) -> Vec2 {
        match self {
            UiElement::Rect(ui_rect) => ui_rect.position,
        }
    }
}

pub struct UiContext {
    pub elements: Arena<UiNode>,
    pub root: Vec<Index>,
}

impl UiContext {
    pub fn new() -> Self {
        Self {
            elements: Arena::new(),
            root: Vec::new(),
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

    pub fn new_elem(&mut self, element: UiElement) -> Index {
        let element = UiNode {
            parent: None,
            children: Vec::new(),
            world_position: element.position(),
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

            let parent_elem = elem.parent.map_or(None, |i| self.elements.get_mut(i));

            let parent_pos = if let Some(parent) = parent_elem {
                parent.world_position
            } else {
                Vec2::ZERO
            };

            let elem = self.elements.get_mut(elem_i).unwrap();
            elem.world_position = parent_pos + elem.element.position();

            for child in elem.children.iter() {
                elements.push(*child);
            }

            match &elem.element {
                UiElement::Rect(ui_rect) => {
                    let color = ui_rect.color;
                    ui_draws.push(UiDraw::Frame(UiFrame {
                        position: elem.world_position,
                        anchor: Vec2::ZERO,
                        size: ui_rect.size,
                        color: Vec4::new(color.x, color.y, color.z, 1.0),
                        texture_id: WHITE_TEXTURE_INDEX,
                        uvs: Default::default(),
                    }));
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use hexxengine::glam::{vec2, vec3};

    use crate::{
        editor_ui::{UiContext, UiRect},
        ui,
    };

    #[test]
    fn build_tree() {
        let mut ctx = UiContext::new();

        let pane = ctx.new_elem(
            UiRect {
                position: vec2(0.0, 0.0),
                size: vec2(100.0, 50.0),
                color: vec3(1.0, 1.0, 1.0),
                ..Default::default()
            }
            .to_elem(),
        );
        ctx.root(pane);

        let inner_pane = ctx.new_elem(
            UiRect {
                size: vec2(100.0, 50.0),
                ..Default::default()
            }
            .to_elem(),
        );
        ctx.parent(inner_pane, pane);

        let mut ui_draws = Vec::new();
        ctx.build_draws(&mut ui_draws);
    }
}
