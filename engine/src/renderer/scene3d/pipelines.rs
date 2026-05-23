use std::fs;

use ash::{prelude::VkResult, vk};

use crate::{
    assets::ASSET_PATH,
    renderer::{
        pipelines::VulkanPipelineBuilder,
        scene3d::{InstanceVertex, MeshVertex},
        vkutils,
    },
};

pub struct Pipelines {
    pub opaque_pipeline: vk::Pipeline,
    pub transparent_pipeline: vk::Pipeline,
    pub shadow_pipeline: vk::Pipeline,
    pub skybox_pipeline: vk::Pipeline,
}

impl Pipelines {
    pub fn new(
        device: &ash::Device,
        scene3d_pipeline_layout: vk::PipelineLayout,
        surface_format: vk::Format,
    ) -> VkResult<Self> {
        let base_pipeline_builder = VulkanPipelineBuilder::default()
            .input_assembly_state(
                vk::PipelineInputAssemblyStateCreateInfo::default()
                    .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
                    .primitive_restart_enable(false),
            )
            .rasterizer(
                vk::PipelineRasterizationStateCreateInfo::default()
                    .depth_clamp_enable(false)
                    .polygon_mode(vk::PolygonMode::FILL)
                    .line_width(1.0)
                    .cull_mode(vk::CullModeFlags::BACK)
                    .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
                    .depth_bias_enable(false),
            )
            .colorblend_state(
                vk::PipelineColorBlendAttachmentState::default()
                    .color_write_mask(vk::ColorComponentFlags::RGBA)
                    .blend_enable(true)
                    .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
                    .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                    .color_blend_op(vk::BlendOp::ADD)
                    .src_alpha_blend_factor(vk::BlendFactor::ONE)
                    .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
                    .alpha_blend_op(vk::BlendOp::ADD),
            )
            .multisampling(
                vk::PipelineMultisampleStateCreateInfo::default()
                    .sample_shading_enable(false)
                    .rasterization_samples(vk::SampleCountFlags::TYPE_1),
            );

        let scene_vert_shader_code = fs::read(format!("{}/main.vert.spv", ASSET_PATH)).unwrap();
        let scene_frag_shader_code = fs::read(format!("{}/main.frag.spv", ASSET_PATH)).unwrap();
        let scene_vert_shader = vkutils::create_shader_module(device, &scene_vert_shader_code)?;
        let scene_frag_shader = vkutils::create_shader_module(device, &scene_frag_shader_code)?;

        let scene_shader_stages = [
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(scene_vert_shader)
                .name(c"main"),
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(scene_frag_shader)
                .name(c"main"),
        ];

        let scene_attribute_descriptions: Vec<vk::VertexInputAttributeDescription> =
            MeshVertex::get_attribute_descriptions()
                .into_iter()
                .chain(InstanceVertex::get_attribute_descriptions())
                .collect();

        let scene_binding_descriptions = [
            MeshVertex::get_binding_description(),
            InstanceVertex::get_binding_description(),
        ];

        let opaque_pipeline_builder = base_pipeline_builder
            .clone()
            .pipeline_layout(scene3d_pipeline_layout)
            .shader_stages(&scene_shader_stages)
            .vertex_input_state(
                vk::PipelineVertexInputStateCreateInfo::default()
                    .vertex_attribute_descriptions(&scene_attribute_descriptions)
                    .vertex_binding_descriptions(&scene_binding_descriptions),
            )
            .depth_stencil_state(
                vk::PipelineDepthStencilStateCreateInfo::default()
                    .depth_test_enable(true)
                    .depth_write_enable(true)
                    .depth_compare_op(vk::CompareOp::GREATER),
            )
            .color_attachment_format(surface_format)
            .depth_attachment_format(vk::Format::D32_SFLOAT);

        let transparent_pipeline_builder = opaque_pipeline_builder.clone().depth_stencil_state(
            vk::PipelineDepthStencilStateCreateInfo::default()
                .depth_test_enable(false)
                .depth_write_enable(false)
                .depth_compare_op(vk::CompareOp::GREATER),
        );

        let shadow_vert_shader_code = fs::read(format!("{}/shadow.vert.spv", ASSET_PATH)).unwrap();
        let shadow_frag_shader_code = fs::read(format!("{}/shadow.frag.spv", ASSET_PATH)).unwrap();

        let shadow_vert_shader =
            vkutils::create_shader_module(device, &shadow_vert_shader_code).unwrap();
        let shadow_frag_shader =
            vkutils::create_shader_module(device, &shadow_frag_shader_code).unwrap();

        let shadow_shader_stages = [
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(shadow_vert_shader)
                .name(c"main"),
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(shadow_frag_shader)
                .name(c"main"),
        ];

        let shadow_pipeline_builder = opaque_pipeline_builder
            .clone()
            .pipeline_layout(scene3d_pipeline_layout)
            .shader_stages(&shadow_shader_stages)
            .no_color_attachments();

        let shadow_vert_shader_code = fs::read(format!("{}/skybox.vert.spv", ASSET_PATH)).unwrap();
        let shadow_frag_shader_code = fs::read(format!("{}/skybox.frag.spv", ASSET_PATH)).unwrap();
        let shadow_vert_shader =
            vkutils::create_shader_module(device, &shadow_vert_shader_code).unwrap();
        let shadow_frag_shader =
            vkutils::create_shader_module(device, &shadow_frag_shader_code).unwrap();

        let shadow_shader_stages = [
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(shadow_vert_shader)
                .name(c"main"),
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(shadow_frag_shader)
                .name(c"main"),
        ];

        let shadow_vertex_attributes = MeshVertex::get_attribute_descriptions();
        let shadow_vertex_bindings = [MeshVertex::get_binding_description()];

        let skybox_pipeline_builder = opaque_pipeline_builder
            .clone()
            .pipeline_layout(scene3d_pipeline_layout)
            .shader_stages(&shadow_shader_stages)
            .vertex_input_state(
                vk::PipelineVertexInputStateCreateInfo::default()
                    .vertex_attribute_descriptions(&shadow_vertex_attributes)
                    .vertex_binding_descriptions(&shadow_vertex_bindings),
            )
            .rasterizer(
                vk::PipelineRasterizationStateCreateInfo::default()
                    .depth_clamp_enable(false)
                    .polygon_mode(vk::PolygonMode::FILL)
                    .line_width(1.0)
                    .cull_mode(vk::CullModeFlags::FRONT)
                    .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
                    .depth_bias_enable(false),
            )
            .colorblend_state(
                vk::PipelineColorBlendAttachmentState::default()
                    .blend_enable(false)
                    .color_write_mask(vk::ColorComponentFlags::RGBA)
                    .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
                    .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                    .color_blend_op(vk::BlendOp::ADD)
                    .src_alpha_blend_factor(vk::BlendFactor::ONE)
                    .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
                    .alpha_blend_op(vk::BlendOp::ADD),
            )
            .depth_stencil_state(
                vk::PipelineDepthStencilStateCreateInfo::default()
                    .depth_test_enable(false)
                    .depth_write_enable(false)
                    .depth_compare_op(vk::CompareOp::GREATER),
            );

        Ok(Self {
            opaque_pipeline: opaque_pipeline_builder.build(device).unwrap(),
            transparent_pipeline: transparent_pipeline_builder.build(device).unwrap(),
            shadow_pipeline: shadow_pipeline_builder.build(device).unwrap(),
            skybox_pipeline: skybox_pipeline_builder.build(device).unwrap(),
        })
    }
}
