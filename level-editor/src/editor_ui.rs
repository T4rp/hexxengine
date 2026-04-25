use hexxengine::glam::vec2;
use hexxengine::{
    glam::{Vec2, Vec3},
    thunderdome::{Arena, Index},
};

struct UiNode {
    parent: Option<Index>,
    children: Vec<Index>,
    world_position: Vec2,
    element: UiElement,
}

#[derive(Default)]
struct UiRect {
    color: Vec3,
    position: Vec2,
    size: Vec2,
}

impl UiRect {
    fn to_elem(self) -> UiElement {
        UiElement::Rect(self)
    }
}

enum UiElement {
    Rect(UiRect),
}

impl UiElement {
    fn position(&self) -> Vec2 {
        match self {
            UiElement::Rect(ui_rect) => ui_rect.position,
        }
    }
}

pub struct UiContext {
    elements: Arena<UiNode>,
}

impl UiContext {
    fn new() -> Self {
        Self {
            elements: Arena::new(),
        }
    }

    fn new_elem(&mut self, element: UiElement, children: &[Index]) -> Index {
        let element = UiNode {
            parent: None,
            children: children.to_vec(),
            world_position: element.position(),
            element,
        };

        let index = self.elements.insert(element);

        for child in children.iter() {
            let element = self.elements.get_mut(*child).unwrap();
            element.parent = Some(index);
        }

        index
    }
}

#[cfg(test)]
mod tests {
    use hexxengine::glam::{vec2, vec3};

    use crate::editor_ui::{UiContext, UiRect};

    #[test]
    fn build_tree() {
        let mut ctx = UiContext::new();

        ctx.new_elem(
            UiRect {
                position: vec2(0.0, 0.0),
                size: vec2(100.0, 50.0),
                color: vec3(0.0, 0.0, 0.0),
                ..Default::default()
            }
            .to_elem(),
            &[],
        );
    }
}
