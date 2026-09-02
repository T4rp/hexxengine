use std::fs;

use ash::{
    prelude::VkResult,
    vk::{self},
};

use crate::{
    assets,
    renderer::{
        pipelines::VulkanPipelineBuilder,
        scene2d::Vertex2d,
        vkutils::{self},
    },
};

pub struct Pipelines {
    pub main_pipeline: vk::Pipeline,
    pub text_pipeline: vk::Pipeline,
}

impl Pipelines {
    pub fn new(
        device: &ash::Device,
        pipeline_layout: vk::PipelineLayout,
        surface_format: vk::Format,
    ) -> VkResult<Pipelines> {
        let vertex_attribute_descriptions = Vertex2d::get_attribute_descriptions();
        let vertex_binding_description = Vertex2d::get_binding_descriptions();

        let base_pipeline_builder = VulkanPipelineBuilder::default()
            .pipeline_layout(pipeline_layout)
            .vertex_input_state(
                vk::PipelineVertexInputStateCreateInfo::default()
                    .vertex_attribute_descriptions(&vertex_attribute_descriptions)
                    .vertex_binding_descriptions(&vertex_binding_description),
            )
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
            )
            .depth_stencil_state(
                vk::PipelineDepthStencilStateCreateInfo::default()
                    .depth_test_enable(false)
                    .depth_write_enable(false)
                    .depth_compare_op(vk::CompareOp::GREATER),
            )
            .color_attachment_format(surface_format)
            .depth_attachment_format(vk::Format::D32_SFLOAT);

        let main_vert_shader_code = fs::read(assets::get_asset_path("main2d.vert.spv")).unwrap();
        let main_frag_shader_code = fs::read(assets::get_asset_path("main2d.frag.spv")).unwrap();

        let main_vert_shader =
            vkutils::create_shader_module(device, &main_vert_shader_code).unwrap();
        let main_frag_shader =
            vkutils::create_shader_module(device, &main_frag_shader_code).unwrap();

        let main_shader_stages = [
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(main_vert_shader)
                .name(c"main"),
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(main_frag_shader)
                .name(c"main"),
        ];

        let main_pipeline = base_pipeline_builder
            .clone()
            .shader_stages(&main_shader_stages);

        let text_vert_shader_code = fs::read(assets::get_asset_path("text.vert.spv")).unwrap();
        let text_frag_shader_code = fs::read(assets::get_asset_path("text.frag.spv")).unwrap();

        let text_vert_shader =
            vkutils::create_shader_module(device, &text_vert_shader_code).unwrap();
        let text_frag_shader =
            vkutils::create_shader_module(device, &text_frag_shader_code).unwrap();

        let text_shader_stages = [
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(text_vert_shader)
                .name(c"main"),
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(text_frag_shader)
                .name(c"main"),
        ];

        let text_pipeline = base_pipeline_builder
            .clone()
            .shader_stages(&text_shader_stages);

        let main_pipeline = main_pipeline.build(device)?;
        let text_pipeline = text_pipeline.build(device)?;

        Ok(Pipelines {
            main_pipeline,
            text_pipeline,
        })
    }
}
