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

#[derive(Default)]
pub struct Frame {
    pub color: Vec4,
    pub position: UiDim,
    pub size: UiDim,
}

impl Frame {
    fn to_elem(self) -> UiElement {
        UiElement::Frame(self)
    }
}

pub struct TextLabel {
    text_box: TextBox,
    text: Cow<'static, str>,
    font: Option<FontHandle>,

    pub color: Vec4,
    pub position: UiDim,
    pub size: UiDim,
}

impl TextLabel {
    fn to_elem(self) -> UiElement {
        UiElement::TextLabel(self)
    }
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
    pub last_child: Option<Index>,
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

    pub fn deparent(&mut self, child_index: Index) {
        let (parent_index, prev_index, next_index) = {
            let child = match self.elements.get(child_index) {
                Some(child) => child,
                None => return,
            };

            match child.parent {
                Some(parent) => (parent, child.prev_sibling, child.next_sibling),
                None => return,
            }
        };

        let parent = match self.elements.get_mut(parent_index) {
            Some(parent) => parent,
            None => return,
        };

        if parent.first_child == Some(child_index) {
            parent.first_child = next_index
        }

        if parent.last_child == Some(child_index) {
            parent.last_child = prev_index
        }

        if let Some(prev_index) = prev_index {
            self.elements.get_mut(prev_index).unwrap().next_sibling = next_index;
        }

        if let Some(next_index) = next_index {
            self.elements.get_mut(next_index).unwrap().prev_sibling = prev_index;
        }

        let child = self.elements.get_mut(child_index).unwrap();
        child.parent = None;
        child.next_sibling = None;
        child.prev_sibling = None;
    }

    pub fn parent(&mut self, parent_index: Index, child_index: Index) {
        self.deparent(child_index);

        let last_child = self
            .elements
            .get(parent_index)
            .and_then(|p| p.last_child)
            .filter(|last_child| self.elements.contains(*last_child));

        let parent = self.elements.get_mut(parent_index).unwrap();
        parent.last_child = Some(child_index);

        if let Some(last_child_index) = last_child {
            self.elements
                .get_mut(last_child_index)
                .unwrap()
                .next_sibling = Some(child_index);
        } else {
            parent.first_child = Some(child_index);
        };

        self.elements.get_mut(child_index).unwrap().parent = Some(parent_index);
    }

    pub fn add_element(&mut self, elem: UiElement) -> Index {
        let ui_node = UiNode {
            world_position: Vec2::ZERO,
            world_scale: Vec2::ZERO,
            parent: None,
            first_child: None,
            last_child: None,
            next_sibling: None,
            prev_sibling: None,
            element: elem,
        };

        self.elements.insert(ui_node)
    }

    pub fn get_element(&self, index: Index) -> Option<&UiNode> {
        self.elements.get(index)
    }

    pub fn get_element_mut(&mut self, index: Index) -> Option<&mut UiNode> {
        self.elements.get_mut(index)
    }
}

#[cfg(test)]
mod tests {
    use crate::ui::{UiTree, ui_tree::Frame};

    #[test]
    fn add_element() {
        let mut ui_tree = UiTree::new();
        let frame = ui_tree.add_element(Frame::default().to_elem());
        assert_eq!(ui_tree.get_element(frame).is_some(), true)
    }

    #[test]
    fn parenting() {
        let mut ui_tree = UiTree::new();
        let frame_idx = ui_tree.add_element(Frame::default().to_elem());
        let child1_idx = ui_tree.add_element(Frame::default().to_elem());
        let child2_idx = ui_tree.add_element(Frame::default().to_elem());

        ui_tree.parent(frame_idx, child1_idx);
        ui_tree.parent(frame_idx, child2_idx);

        let frame = ui_tree.get_element(frame_idx).unwrap();
        let child1 = ui_tree.get_element(child1_idx).unwrap();
        let child2 = ui_tree.get_element(child2_idx).unwrap();

        assert!(child1.parent.is_some());
        assert_eq!(child1.parent.unwrap(), frame_idx);

        assert!(child2.parent.is_some());
        assert_eq!(child2.parent.unwrap(), frame_idx);

        assert!(frame.first_child.is_some());
        assert_eq!(frame.first_child.unwrap(), child1_idx);

        assert!(frame.last_child.is_some());
        assert_eq!(frame.last_child.unwrap(), child2_idx);
    }
}
