use glam::{Mat4, Quat, Vec2, Vec3, Vec4, Vec4Swizzles, vec2, vec4};
use thunderdome::Index;

use crate::{
    camera::Camera,
    renderer::{scene2d::Vertex2d, scene3d::MeshVertex},
    text::{FontHandle, GlyphPositions, TextBox},
};

pub struct Lighting {
    pub sun_direction: Vec3,
    pub sun_color: Vec3,
    pub sun_power: f32,
    pub ambient_color: Vec3,
    pub skybox_id: Index,
}

#[derive(Clone)]
pub struct MeshNode {
    pub position: Vec3,
    pub orientation: Quat,
    pub size: Vec3,
    pub color: Vec3,
    pub opacity: f32,
    pub mesh_id: Index,
    pub material_id: Index,
}

pub struct UiFrame {
    pub position: Vec2,
    pub size: Vec2,
    pub color: Vec4,
    pub texture_id: Index,
    pub uvs: [Vec2; 4],
}

impl UiFrame {
    pub fn new(position: Vec2, size: Vec2, texture_id: Index) -> Self {
        Self {
            position,
            size,
            color: Vec4::new(0.0, 0.0, 0.0, 1.0),
            texture_id,
            uvs: [
                vec2(0.0, 0.0),
                vec2(0.0, 1.0),
                vec2(1.0, 1.0),
                vec2(1.0, 0.0),
            ],
        }
    }

    pub fn push_verts(&self, vertices: &mut Vec<Vertex2d>, indices: &mut Vec<u16>) -> (u32, u32) {
        let vert_offset = vertices.len();

        vertices.push(Vertex2d {
            pos: self.position,
            uv: self.uvs[0],
            color: self.color,
        });

        vertices.push(Vertex2d {
            pos: vec2(self.position.x, self.position.y + self.size.y),
            uv: self.uvs[1],
            color: self.color,
        });

        vertices.push(Vertex2d {
            pos: vec2(self.position.x + self.size.x, self.position.y + self.size.y),
            uv: self.uvs[2],
            color: self.color,
        });

        vertices.push(Vertex2d {
            pos: vec2(self.position.x + self.size.x, self.position.y),
            uv: self.uvs[3],
            color: self.color,
        });

        indices.push(vert_offset as u16);
        indices.push(vert_offset as u16 + 1);
        indices.push(vert_offset as u16 + 2);

        indices.push(vert_offset as u16);
        indices.push(vert_offset as u16 + 2);
        indices.push(vert_offset as u16 + 3);

        (4, 6)
    }
}

pub struct UiText {
    pub font: FontHandle,
    pub position: Vec2,
    pub font_height: u32,
    pub glyph_positions: Vec<GlyphPositions>,
    pub color: Vec3,
}

impl UiText {
    pub fn new(
        font: FontHandle,
        position: Vec2,
        height: u32,
        glyph_positions: Vec<GlyphPositions>,
    ) -> Self {
        Self {
            font,
            position,
            font_height: height,
            glyph_positions,
            color: Vec3::ZERO,
        }
    }

    pub fn from_text_box(text_box: &TextBox, position: Vec2) -> Self {
        Self::new(
            text_box.font,
            position,
            text_box.font_height,
            text_box.glyph_positions.clone(),
        )
    }
}

pub enum UiDraw {
    Frame(UiFrame),
    Text(UiText),
}

pub struct RenderScene {
    pub camera: Camera,
    pub meshes: Vec<MeshNode>,
    pub ui: Vec<UiDraw>,
    pub lighting: Lighting,
}

impl RenderScene {
    pub fn new(camera: Camera, lighting: Lighting) -> Self {
        Self {
            camera,
            meshes: Vec::new(),
            ui: Vec::new(),
            lighting,
        }
    }

    pub fn push_ui_frame(&mut self, frame: UiFrame) {
        self.ui.push(UiDraw::Frame(frame));
    }

    pub fn push_ui_text(&mut self, ui_text: UiText) {
        self.ui.push(UiDraw::Text(ui_text));
    }
}
