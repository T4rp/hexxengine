use std::{borrow::Cow, collections::VecDeque};

use glam::{Vec2, Vec3, Vec4, Vec4Swizzles};
use thunderdome::{Arena, Index};
use winit::event::MouseButton;

use crate::{
    input::{InputEvent, InputHandler, InputState},
    renderer::renderer::WHITE_TEXTURE_INDEX,
    scene::{UiDraw, UiFrame, UiText},
    text::{FontHandle, FontManager, TextBox},
    ui::{Element, UiDim, UiElement},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiEventType {
    Pressed,
}

pub struct UiEvent {
    pub input_event: Option<InputEvent>,
    pub event_type: UiEventType,
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
    pub element: Element,
    pub events: Option<Vec<UiEvent>>,
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

    pub fn set_root_size(&mut self, root_size: Vec2) {
        self.root_size = root_size;
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

    pub fn enable_events(&mut self, elem_index: Index) {
        let elem = self.elements.get_mut(elem_index).unwrap();

        if elem.events.is_none() {
            elem.events = Some(Vec::new())
        }
    }

    pub fn disable_events(&mut self, elem_index: Index) {
        let elem = self.elements.get_mut(elem_index).unwrap();

        if elem.events.is_some() {
            elem.events = None
        }
    }

    pub fn add_element<T: UiElement>(&mut self, elem: T) -> Index {
        let ui_node = UiNode {
            world_position: Vec2::ZERO,
            world_size: Vec2::ZERO,
            dimensions_dirty: true,
            parent: None,
            first_child: None,
            last_child: None,
            next_sibling: None,
            prev_sibling: None,
            element: elem.to_enum(),
            events: None,
        };

        self.elements.insert(ui_node)
    }

    pub fn remove(&mut self, elem_index: Index) {
        self.deparent(elem_index);

        // PERF: maybe find a better way
        if let Some(index) = self.roots.iter().position(|i| *i == elem_index) {
            self.roots.swap_remove(index);
        }

        let element = self.elements.get(elem_index).unwrap();
        let mut current_child = element.first_child;

        while let Some(child) = current_child {
            current_child = self.elements.get(child).and_then(|e| e.next_sibling);
            self.remove(child);
        }

        self.elements.remove(elem_index);
    }

    pub fn draw(&mut self, font_manager: &mut FontManager, ui_draws: &mut Vec<UiDraw>) {
        let mut elements: VecDeque<Index> = VecDeque::from(self.roots.clone());

        while let Some(node_index) = elements.pop_front() {
            let node = self
                .elements
                .get(node_index)
                .expect("root element not in arena");

            let visible = node.element.visible();

            if !visible {
                continue;
            }

            let position = node.element.position();
            let size = node.element.size();
            let anchor = node.element.anchor();

            let parent_node = node.parent.and_then(|i| self.elements.get(i));

            let (parent_pos, parent_size) = if let Some(parent) = parent_node {
                (parent.world_position, parent.world_size)
            } else {
                (Vec2::ZERO, self.root_size)
            };

            let world_size = parent_size * size.scale + size.offset;
            let world_position =
                parent_pos + parent_size * position.scale + position.offset - world_size * anchor;

            // TODO: only calc this when dirty
            let node = self.elements.get_mut(node_index).unwrap();
            node.world_position = world_position;
            node.world_size = world_size;

            let mut current_node = node.first_child;

            while let Some(index) = current_node {
                elements.push_back(index);
                current_node = self.elements.get(index).and_then(|node| node.next_sibling)
            }

            match &mut self.elements.get_mut(node_index).unwrap().element {
                Element::Frame(frame) => {
                    ui_draws.push(UiDraw::Frame(UiFrame {
                        position: world_position,
                        size: world_size,
                        color: frame.color,
                        texture_id: WHITE_TEXTURE_INDEX,
                        uvs: Default::default(),
                    }));
                }
                Element::TextLabel(text_label) => {
                    if text_label.text_box.size != world_size {
                        text_label.text_box.set_size(world_size);
                    }
                    text_label.text_box.calculate_layout(font_manager);

                    if text_label.background_color.w != 0.0 {
                        // PERF: This does not benefit from batching by breadth.
                        ui_draws.push(UiDraw::Frame(UiFrame {
                            position: world_position,
                            size: world_size,
                            color: text_label.background_color,
                            texture_id: WHITE_TEXTURE_INDEX,
                            uvs: Default::default(),
                        }));
                    }

                    ui_draws.push(UiDraw::Text(UiText {
                        // TODO: have some way to do default font
                        font: text_label.font.unwrap(),
                        position: world_position,
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

    pub fn clear_events(&mut self, elem_index: Index) {
        let node = self.elements.get_mut(elem_index).unwrap();

        if let Some(events) = node.events.as_mut() {
            events.clear();
        }
    }

    pub fn handle_input(&mut self, input_handler: &InputHandler) {
        let Some((button, state, position)) =
            input_handler
                .get_input_events()
                .iter()
                .find_map(|event| match event {
                    InputEvent::MouseButtonEvent {
                        button,
                        state,
                        position,
                    } => Some((button, state, position)),
                    _ => None,
                })
        else {
            return;
        };

        if *button != MouseButton::Left || *state != InputState::Pressed {
            return;
        }

        let mut elements = self.roots.clone();

        while let Some(elem_index) = elements.pop() {
            let node = self.elements.get(elem_index).unwrap();

            if !node.element.visible() {
                continue;
            }

            let mut current_node = node.first_child;

            while let Some(index) = current_node {
                elements.push(index);
                current_node = self.elements.get(index).and_then(|node| node.next_sibling)
            }

            let node = self.elements.get_mut(elem_index).unwrap();

            let Some(events) = node.events.as_mut() else {
                continue;
            };

            let top_left = node.world_position;
            let bottom_right = node.world_position + node.world_size;

            if position.cmpge(top_left).all() && position.cmple(bottom_right).all() {
                events.push(UiEvent {
                    input_event: None,
                    event_type: UiEventType::Pressed,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use glam::{Vec2, Vec3};

    use crate::{
        assets,
        input::{InputEvent::MouseButtonEvent, InputHandler},
        text::{FontManager, TextBox},
        ui::{self, Frame, TextLabel, UiElement, UiTree},
    };

    use super::UiDim;

    #[test]
    fn add_element() {
        let mut ui_tree = UiTree::new();
        let frame = ui_tree.add_element(Frame::default());
        assert_eq!(ui_tree.get_element(frame).is_some(), true)
    }

    #[test]
    fn parenting() {
        let mut ui_tree = UiTree::new();
        let frame_idx = ui_tree.add_element(Frame::default());
        let child1_idx = ui_tree.add_element(Frame::default());
        let child2_idx = ui_tree.add_element(Frame::default());

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
    fn deparenting() {
        let mut ui_tree = UiTree::new();
        let frame_idx = ui_tree.add_element(Frame::default());
        let child1_idx = ui_tree.add_element(Frame::default());
        let child2_idx = ui_tree.add_element(Frame::default());
        let child3_idx = ui_tree.add_element(Frame::default());

        ui_tree.parent(frame_idx, child1_idx);
        ui_tree.parent(frame_idx, child2_idx);
        ui_tree.parent(frame_idx, child3_idx);

        ui_tree.deparent(child2_idx);
        ui_tree.deparent(child1_idx);
        ui_tree.deparent(child3_idx);
    }

    #[test]
    fn removing() {
        let mut ui_tree = UiTree::new();
        let frame_idx = ui_tree.add_element(Frame::default());
        let child1_idx = ui_tree.add_element(Frame::default());

        ui_tree.parent(frame_idx, child1_idx);
        ui_tree.remove(frame_idx);

        assert!(ui_tree.get_element(frame_idx).is_none());
        assert!(ui_tree.get_element(child1_idx).is_none());
    }

    #[test]
    fn draw_tree_frames() {
        let mut font_manager = FontManager::new();

        let mut ui_tree = UiTree::new();
        let frame_idx = ui_tree.add_element(Frame::default());
        let child1_idx = ui_tree.add_element(Frame::default());
        let child2_idx = ui_tree.add_element(Frame::default());

        ui_tree.root(frame_idx);
        ui_tree.parent(frame_idx, child1_idx);
        ui_tree.parent(frame_idx, child2_idx);

        let mut draws = Vec::new();

        ui_tree.draw(&mut font_manager, &mut draws);
    }

    #[test]
    fn draw_tree_text() {
        let mut font_manager = FontManager::new();
        let font_data = fs::read(assets::get_asset_path("unifont-17.0.03.otf")).unwrap();
        let font_handle = font_manager.load_font(&font_data).unwrap();

        let mut ui_tree = UiTree::new();
        let frame_idx = ui_tree.add_element(Frame::default());
        let label_idx = ui_tree.add_element(TextLabel::new(font_handle));

        ui_tree.root(frame_idx);
        ui_tree.parent(frame_idx, label_idx);

        let mut draws = Vec::new();
        ui_tree.draw(&mut font_manager, &mut draws);

        assert!(draws.len() > 0);
    }

    #[test]
    fn draw_tree_not_visible() {
        let mut font_manager = FontManager::new();
        let font_data = fs::read(assets::get_asset_path("unifont-17.0.03.otf")).unwrap();
        let font_handle = font_manager.load_font(&font_data).unwrap();

        let mut ui_tree = UiTree::new();

        let frame_idx = ui_tree.add_element(Frame {
            visible: false,
            ..Default::default()
        });

        let child1_idx = ui_tree.add_element(Frame::default());

        ui_tree.root(frame_idx);
        ui_tree.parent(frame_idx, child1_idx);

        let mut draws = Vec::new();
        ui_tree.draw(&mut font_manager, &mut draws);

        assert_eq!(draws.len(), 0);
    }

    #[test]
    fn click_events() {
        let mut font_manager = FontManager::new();
        let font_data = fs::read(assets::get_asset_path("unifont-17.0.03.otf")).unwrap();
        let _font_handle = font_manager.load_font(&font_data).unwrap();

        let mut input_handler = InputHandler::new();
        let mut ui_tree = UiTree::new();

        ui_tree.set_root_size(Vec2::new(100.0, 100.0));

        let frame_idx = ui_tree.add_element(Frame {
            position: UiDim::new(0.5, 0.5, 0.0, 0.0),
            anchor: Vec2::new(0.5, 0.5),
            size: UiDim::new(0.0, 0.0, 20.0, 20.0),
            ..Default::default()
        });

        ui_tree.root(frame_idx);
        ui_tree.enable_events(frame_idx);

        let mut draws = Vec::new();
        ui_tree.draw(&mut font_manager, &mut draws);

        input_handler.simulate_event(MouseButtonEvent {
            button: winit::event::MouseButton::Left,
            state: crate::input::InputState::Pressed,
            position: Vec2::new(50.0, 50.0),
        });

        ui_tree.handle_input(&input_handler);

        let node = ui_tree.get_element(frame_idx).unwrap();

        assert_eq!(node.events.is_some(), true);
        assert!(node.events.as_ref().unwrap().len() >= 1);
    }
}
