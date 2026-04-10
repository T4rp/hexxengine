use std::mem;

use ash::vk;
use glam::{Mat4, Vec2, Vec4};

#[derive(Clone, Copy)]
#[repr(C)]
pub struct CameraUniform2d {
    pub proj: Mat4,
    pub view: Mat4,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct Vertex2d {
    pub pos: Vec2,
    pub uv: Vec2,
    pub color: Vec4,
}

impl Vertex2d {
    pub fn new_solid(pos: Vec2, color: Vec4) -> Vertex2d {
        Self {
            pos,
            uv: Vec2::ZERO,
            color,
        }
    }

    pub fn get_attribute_descriptions() -> [vk::VertexInputAttributeDescription; 3] {
        [
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(0)
                .format(vk::Format::R32G32_SFLOAT)
                .offset(mem::offset_of!(Vertex2d, pos) as u32),
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(1)
                .format(vk::Format::R32G32_SFLOAT)
                .offset(mem::offset_of!(Vertex2d, uv) as u32),
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(2)
                .format(vk::Format::R32G32B32A32_SFLOAT)
                .offset(mem::offset_of!(Vertex2d, color) as u32),
        ]
    }

    pub fn get_binding_descriptions() -> [vk::VertexInputBindingDescription; 1] {
        [vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(mem::size_of::<Vertex2d>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)]
    }
}
