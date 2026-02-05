use std::mem;

use ash::vk;
use glam::{Mat3, Mat4, Quat, Vec2, Vec3, Vec4};

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

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct MeshVertex {
    pub pos: Vec3,
    pub norm: Vec3,
    pub uv: Vec2,
}

impl MeshVertex {
    pub fn get_attribute_descriptions() -> [vk::VertexInputAttributeDescription; 3] {
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
                .location(2)
                .format(vk::Format::R32G32_SFLOAT)
                .offset(mem::offset_of!(MeshVertex, uv) as u32),
        ]
    }

    pub fn get_binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(mem::size_of::<MeshVertex>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
    }
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct InstanceVertex {
    pub model: Mat4,
    pub model_normal: Mat3,
    pub color: Vec3,
}

impl InstanceVertex {
    pub fn new(position: Vec3, rotation: Quat, size: Vec3, color: Vec3) -> Self {
        let model = Mat4::from_scale_rotation_translation(size, rotation, position);
        let model_normal = Mat3::from_mat4(model.inverse().transpose());

        Self {
            model,
            model_normal,
            color,
        }
    }

    pub fn get_attribute_descriptions() -> [vk::VertexInputAttributeDescription; 8] {
        [
            vk::VertexInputAttributeDescription::default()
                .binding(1)
                .location(3)
                .format(vk::Format::R32G32B32A32_SFLOAT)
                .offset(mem::offset_of!(InstanceVertex, model.x_axis) as u32),
            vk::VertexInputAttributeDescription::default()
                .binding(1)
                .location(4)
                .format(vk::Format::R32G32B32A32_SFLOAT)
                .offset(mem::offset_of!(InstanceVertex, model.y_axis) as u32),
            vk::VertexInputAttributeDescription::default()
                .binding(1)
                .location(5)
                .format(vk::Format::R32G32B32A32_SFLOAT)
                .offset(mem::offset_of!(InstanceVertex, model.z_axis) as u32),
            vk::VertexInputAttributeDescription::default()
                .binding(1)
                .location(6)
                .format(vk::Format::R32G32B32A32_SFLOAT)
                .offset(mem::offset_of!(InstanceVertex, model.w_axis) as u32),
            vk::VertexInputAttributeDescription::default()
                .binding(1)
                .location(7)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(mem::offset_of!(InstanceVertex, model_normal.x_axis) as u32),
            vk::VertexInputAttributeDescription::default()
                .binding(1)
                .location(8)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(mem::offset_of!(InstanceVertex, model_normal.y_axis) as u32),
            vk::VertexInputAttributeDescription::default()
                .binding(1)
                .location(9)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(mem::offset_of!(InstanceVertex, model_normal.z_axis) as u32),
            vk::VertexInputAttributeDescription::default()
                .binding(1)
                .location(10)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(mem::offset_of!(InstanceVertex, color) as u32),
        ]
    }

    pub fn get_binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::default()
            .binding(1)
            .stride(mem::size_of::<InstanceVertex>() as u32)
            .input_rate(vk::VertexInputRate::INSTANCE)
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct CameraUniform {
    pub proj: Mat4,
    pub view: Mat4,
    pub camera_position: Vec4,
    pub light_proj: Mat4,
    pub light_view: Mat4,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct SceneUniform {
    pub sun_direction: Vec4,
    pub sun_color: Vec4,
    pub ambient_color: Vec4,
}

#[repr(C)]
pub struct MaterialUniform {
    uv_scale: Vec2,
    shininess: f32,
    flags: u32,
}
