use std::{
    any::{self, Any, TypeId},
    borrow::Cow,
    collections::HashMap,
    rc::Rc,
};

use glam::{IVec2, Mat4, Quat, Vec2, Vec3, Vec4, Vec4Swizzles, vec2, vec4};
use thunderdome::{Arena, Index};

use crate::{
    renderer::{
        buffer_objects::{scene2d::Vertex2d, scene3d::MeshVertex},
        renderer::MeshHandle,
    },
    text::GlyphAtlas,
};

pub struct Camera {
    pub position: Vec3,
    pub orientation: Quat,
    pub fov: f32,
}

const NDC_CORNERS: &[Vec4] = &[
    vec4(-1.0, -1.0, 1.0, 1.0),
    vec4(1.0, -1.0, 1.0, 1.0),
    vec4(-1.0, 1.0, 1.0, 1.0),
    vec4(1.0, 1.0, 1.0, 1.0),
    vec4(-1.0, -1.0, 0.0, 1.0),
    vec4(1.0, -1.0, 0.0, 1.0),
    vec4(-1.0, 1.0, 0.0, 1.0),
    vec4(1.0, 1.0, 0.0, 1.0),
];

impl Camera {
    pub fn new(position: Vec3, orientation: Quat, fov: f32) -> Self {
        Self {
            position,
            orientation,
            fov,
        }
    }

    pub fn calc_perspective_matrices(&self, aspect_ratio: f32) -> (Mat4, Mat4) {
        let vertical_fov = 2.0 * (self.fov.to_radians() * 0.5).tan().atan2(aspect_ratio);

        let mut proj = Mat4::perspective_infinite_reverse_rh(vertical_fov, aspect_ratio, 1.0);
        proj.y_axis *= vec4(1.0, -1.0, 1.0, 1.0);

        let view = Mat4::from_rotation_translation(self.orientation, self.position).inverse();

        (proj, view)
    }

    pub fn calc_frustrum_corners(&self, aspect_ratio: f32, near: f32, far: f32) -> [Vec3; 8] {
        let vertical_fov = 2.0 * (self.fov.to_radians() * 0.5).tan().atan2(aspect_ratio);

        let mut proj = Mat4::perspective_rh(vertical_fov, aspect_ratio, near, far);
        proj.y_axis *= vec4(1.0, -1.0, 1.0, 1.0);

        let view = Mat4::from_rotation_translation(self.orientation, self.position).inverse();

        let mut corners = [Vec3::ZERO; 8];

        let inv_pv = (proj * view).inverse();
        for i in 0..8 {
            let corner4 = inv_pv * NDC_CORNERS[i];
            corners[i] = corner4.xyz() / corner4.w
        }

        corners
    }
}

pub struct Lighting {
    pub sun_direction: Vec3,
    pub sun_color: Vec3,
    pub sun_power: f32,
    pub ambient_color: Vec3,
    pub skybox_id: u32,
}

#[derive(Clone)]
pub struct MeshNode {
    pub position: Vec3,
    pub orientation: Quat,
    pub size: Vec3,
    pub color: Vec3,
    pub opacity: f32,
    pub mesh_id: MeshHandle,
    pub material_id: u32,
}

pub struct UiFrame {
    pub position: Vec2,
    pub size: Vec2,
    pub color: Vec4,
    pub texture_id: u32,
    pub uvs: [Vec2; 4],
}

impl UiFrame {
    pub fn new(position: Vec2, size: Vec2, texture_id: u32) -> Self {
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
    pub position: IVec2,
    pub font_height: u32,
    pub text: Cow<'static, str>,
    pub color: Vec3,
}

impl UiText {
    pub fn new(position: IVec2, height: u32, text: impl Into<Cow<'static, str>>) -> Self {
        Self {
            position,
            font_height: height,
            text: text.into(),
            color: Vec3::ZERO,
        }
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

pub struct MeshData {
    pub vertices: Vec<MeshVertex>,
    pub indices: Vec<u16>,
}
