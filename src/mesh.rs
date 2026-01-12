use std::mem;

use ash::vk;
use glam::{Mat4, Quat, Vec2, Vec3};

#[derive(Clone, Copy)]
#[repr(C)]
pub struct Vertex2d {
    pub pos: Vec2,
    pub uv: Vec2,
    pub color: Vec3,
}

impl Vertex2d {
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
                .format(vk::Format::R32G32B32_SFLOAT)
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

#[derive(Clone, Copy)]
#[repr(C)]
pub struct MeshVertex {
    pub pos: Vec3,
    pub norm: Vec3,
    pub uv: Vec2,
}

impl MeshVertex {
    fn get_attribute_descriptions() -> [vk::VertexInputAttributeDescription; 3] {
        [
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(0)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(mem::offset_of!(MeshVertex, pos) as u32),
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(1)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(mem::offset_of!(MeshVertex, norm) as u32),
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(1)
                .format(vk::Format::R32G32_SFLOAT)
                .offset(mem::offset_of!(MeshVertex, uv) as u32),
        ]
    }

    fn get_binding_descriptions() -> [vk::VertexInputBindingDescription; 1] {
        [vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(mem::size_of::<MeshVertex>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)]
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct CameraUniform {
    pub proj: Mat4,
    pub view: Mat4,
}

impl CameraUniform {
    pub fn new(position: Vec3, orientation: Quat, fov: f32, aspect_ratio: f32) -> Self {
        let proj = Mat4::perspective_infinite_reverse_rh(fov.to_radians(), aspect_ratio, 1.0);
        let view = Mat4::from_rotation_translation(orientation, position);

        Self { proj, view }
    }
}
