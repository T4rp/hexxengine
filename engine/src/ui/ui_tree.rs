use std::borrow::Cow;

use glam::{Vec2, Vec3, Vec4, Vec4Swizzles};
use thunderdome::{Arena, Index};

use crate::{
    renderer::renderer::WHITE_TEXTURE_INDEX,
    scene::{UiDraw, UiFrame, UiText},
    text::{FontHandle, FontManager, TextBox},
};

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
    font_height: u32,
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

    pub fn to_elem(self) -> UiElement {
        UiElement::TextLabel(self)
    }
}

pub enum UiElement {
    Frame(Frame),
    TextLabel(TextLabel),
}

impl UiElement {
    fn position(&self) -> UiDim {
        match self {
            UiElement::Frame(frame) => frame.position,
            UiElement::TextLabel(text_label) => text_label.position,
        }
    }

    fn size(&self) -> UiDim {
        match self {
            UiElement::Frame(frame) => frame.size,
            UiElement::TextLabel(text_label) => text_label.size,
        }
    }
}

pub struct UiNode {
    pub world_position: Vec2,
    pub world_size: Vec2,
    pub dimensions_dirty: bool,

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
    is_dirty: bool,
}

impl UiTree {
    pub fn new() -> Self {
        Self {
            elements: Arena::new(),
            roots: Vec::new(),
            root_size: Vec2::ZERO,
            is_dirty: true,
        }
    }

    pub fn root(&mut self, element_index: Index) {
        self.deparent(element_index);
        self.roots.push(element_index);
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
            world_size: Vec2::ZERO,
            dimensions_dirty: true,
            parent: None,
            first_child: None,
            last_child: None,
            next_sibling: None,
            prev_sibling: None,
            element: elem,
        };

        self.elements.insert(ui_node)
    }

    pub fn draw(&mut self, font_manager: &mut FontManager, ui_draws: &mut Vec<UiDraw>) {
        let mut elements = self.roots.clone();

        while let Some(node_index) = elements.pop() {
            let node = self
                .elements
                .get(node_index)
                .expect("root element not in arena");

            let position = node.element.position();
            let size = node.element.size();

            let parent_node = node.parent.and_then(|i| self.elements.get(i));

            let (parent_pos, parent_size) = if let Some(parent) = parent_node {
                (parent.world_position, parent.world_size)
            } else {
                (Vec2::ZERO, self.root_size)
            };

            let world_position = parent_pos + parent_size * position.scale + position.offset;
            let world_size = parent_size * size.scale + size.offset;

            // TODO: only calc this when dirty
            let node = self.elements.get_mut(node_index).unwrap();
            node.world_position = world_position;
            node.world_size = world_size;

            let mut current_node = node.first_child;

            while let Some(index) = current_node {
                elements.push(index);
                current_node = self.elements.get(index).and_then(|node| node.next_sibling)
            }

            match &mut self.elements.get_mut(node_index).unwrap().element {
                UiElement::Frame(frame) => {
                    ui_draws.push(UiDraw::Frame(UiFrame {
                        position: world_position,
                        size: world_size,
                        anchor: Vec2::ZERO,
                        color: frame.color,
                        texture_id: WHITE_TEXTURE_INDEX,
                        uvs: Default::default(),
                    }));
                }
                UiElement::TextLabel(text_label) => {
                    text_label.text_box.calculate_layout(font_manager);

                    ui_draws.push(UiDraw::Text(UiText {
                        // TODO: have some way to do default font
                        font: text_label.font.unwrap(),
                        position: world_position,
                        anchor: Vec2::ZERO,
                        font_height: text_label.font_height,
                        glyph_positions: text_label.text_box.glyph_positions.clone(),
                        color: text_label.color.xyz(),
                    }));
                }
            }
        }
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
    use std::fs;

    use glam::Vec3;

    use crate::{
        assets::ASSET_PATH,
        text::{FontManager, TextBox},
        ui::{
            UiTree,
            ui_tree::{Frame, TextLabel},
        },
    };

    use super::UiDim;

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

    #[test]
    fn draw_tree_frames() {
        let mut font_manager = FontManager::new();

        let mut ui_tree = UiTree::new();
        let frame_idx = ui_tree.add_element(Frame::default().to_elem());
        let child1_idx = ui_tree.add_element(Frame::default().to_elem());
        let child2_idx = ui_tree.add_element(Frame::default().to_elem());

        ui_tree.root(frame_idx);
        ui_tree.parent(frame_idx, child1_idx);
        ui_tree.parent(frame_idx, child2_idx);

        let mut draws = Vec::new();

        ui_tree.draw(&mut font_manager, &mut draws);
    }

    #[test]
    fn draw_tree_text() {
        let mut font_manager = FontManager::new();
        let font_data = fs::read(format!("{}/unifont-17.0.03.otf", ASSET_PATH)).unwrap();
        let font_handle = font_manager.load_font(&font_data).unwrap();

        let mut ui_tree = UiTree::new();
        let frame_idx = ui_tree.add_element(Frame::default().to_elem());
        let label_idx = ui_tree.add_element(TextLabel::new(font_handle).to_elem());

        ui_tree.root(frame_idx);
        ui_tree.parent(frame_idx, label_idx);

        let mut draws = Vec::new();

        ui_tree.draw(&mut font_manager, &mut draws);
    }
}
