use std::{fs, io::Cursor};

use ash::vk::{self, PipelineLayout};

use crate::{
    assets::ASSET_PATH,
    renderer::mesh::{InstanceVertex, MeshVertex, Vertex2d},
};

fn create_shader_module(device: &ash::Device, data: &[u8]) -> vk::ShaderModule {
    let mut cursor = Cursor::new(data);
    let spv = ash::util::read_spv(&mut cursor).unwrap();

    let create_info = vk::ShaderModuleCreateInfo::default().code(&spv);

    unsafe { device.create_shader_module(&create_info, None).unwrap() }
}

pub struct RendererPipelineObjects {
    pub main_2d_graphics_pipeline: vk::Pipeline,
    pub main_graphics_pipeline: vk::Pipeline,
    pub shadow_graphics_pipeline: vk::Pipeline,
    pub skybox_graphics_pipeline: vk::Pipeline,
    pub main_transparent_graphics_pipeline: vk::Pipeline,
}

impl RendererPipelineObjects {
    pub fn new(
        device: &ash::Device,
        pipeline_layout_3d: vk::PipelineLayout,
        pipeline_layout_2d: vk::PipelineLayout,
        surface_format: vk::SurfaceFormatKHR,
    ) -> Self {
        let main_graphics_pipeline =
            create_main_graphics_pipeline(device, pipeline_layout_3d, surface_format);

        let skybox_graphics_pipeline =
            create_sky_graphics_pipeline(device, pipeline_layout_3d, surface_format);

        let main_transparent_graphics_pipeline =
            create_main_transparent_graphics_pipeline(device, pipeline_layout_3d, surface_format);

        let shadow_graphics_pipeline = create_shadow_graphics_pipeline(device, pipeline_layout_3d);

        let main_2d_graphics_pipeline =
            create_main_2d_graphics_pipeline(device, pipeline_layout_2d, surface_format);

        Self {
            main_2d_graphics_pipeline,
            main_graphics_pipeline,
            shadow_graphics_pipeline,
            skybox_graphics_pipeline,
            main_transparent_graphics_pipeline,
        }
    }
}

fn create_main_2d_graphics_pipeline(
    device: &ash::Device,
    pipeline_layout: vk::PipelineLayout,
    surface_format: vk::SurfaceFormatKHR,
) -> vk::Pipeline {
    let dynamic_states = &[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];

    let dynamic_state_info =
        vk::PipelineDynamicStateCreateInfo::default().dynamic_states(dynamic_states);

    let viewports = &[vk::Viewport::default()];
    let scissors = &[vk::Rect2D::default()];

    let viewport_state_info = vk::PipelineViewportStateCreateInfo::default()
        .viewports(viewports)
        .scissors(scissors);

    let vert_shader_code = fs::read(format!("{}/main2d.vert.spv", ASSET_PATH)).unwrap();
    let frag_shader_code = fs::read(format!("{}/main2d.frag.spv", ASSET_PATH)).unwrap();

    let vertex_shader = create_shader_module(device, &vert_shader_code);
    let fragment_shader = create_shader_module(device, &frag_shader_code);

    let vert_stage_info = vk::PipelineShaderStageCreateInfo::default()
        .stage(vk::ShaderStageFlags::VERTEX)
        .module(vertex_shader)
        .name(c"main");

    let frag_stage_info = vk::PipelineShaderStageCreateInfo::default()
        .stage(vk::ShaderStageFlags::FRAGMENT)
        .module(fragment_shader)
        .name(c"main");

    let shader_stages = &[vert_stage_info, frag_stage_info];

    let vertex_attribute_descriptions = Vertex2d::get_attribute_descriptions();
    let vertex_binding_description = Vertex2d::get_binding_descriptions();

    let vertex_input_state_info = vk::PipelineVertexInputStateCreateInfo::default()
        .vertex_attribute_descriptions(&vertex_attribute_descriptions)
        .vertex_binding_descriptions(&vertex_binding_description);

    let input_assembly_state_info = vk::PipelineInputAssemblyStateCreateInfo::default()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
        .primitive_restart_enable(false);

    let rasterization_info = vk::PipelineRasterizationStateCreateInfo::default()
        .depth_clamp_enable(false)
        .polygon_mode(vk::PolygonMode::FILL)
        .line_width(1.0)
        .cull_mode(vk::CullModeFlags::BACK)
        .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
        .depth_bias_enable(false);

    let multisample_info = vk::PipelineMultisampleStateCreateInfo::default()
        .sample_shading_enable(false)
        .rasterization_samples(vk::SampleCountFlags::TYPE_1);

    let color_blend_attachment_states = &[vk::PipelineColorBlendAttachmentState::default()
        .color_write_mask(vk::ColorComponentFlags::RGBA)
        .blend_enable(true)
        .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
        .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
        .color_blend_op(vk::BlendOp::ADD)
        .src_alpha_blend_factor(vk::BlendFactor::ONE)
        .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
        .alpha_blend_op(vk::BlendOp::ADD)];

    let color_blender_state_info =
        vk::PipelineColorBlendStateCreateInfo::default().attachments(color_blend_attachment_states);

    let depth_stencil_state_info = vk::PipelineDepthStencilStateCreateInfo::default()
        .depth_test_enable(false)
        .depth_write_enable(false)
        .depth_compare_op(vk::CompareOp::NEVER);

    let color_attachment_formats = [surface_format.format];
    let mut rendering_create_info = vk::PipelineRenderingCreateInfo::default()
        .color_attachment_formats(&color_attachment_formats);

    let graphics_pipeline_create_info = &[vk::GraphicsPipelineCreateInfo::default()
        .stages(shader_stages)
        .vertex_input_state(&vertex_input_state_info)
        .input_assembly_state(&input_assembly_state_info)
        .dynamic_state(&dynamic_state_info)
        .viewport_state(&viewport_state_info)
        .rasterization_state(&rasterization_info)
        .multisample_state(&multisample_info)
        .color_blend_state(&color_blender_state_info)
        .layout(pipeline_layout)
        .depth_stencil_state(&depth_stencil_state_info)
        .subpass(0)
        .push_next(&mut rendering_create_info)];

    let graphics_pipeline = unsafe {
        device
            .create_graphics_pipelines(
                vk::PipelineCache::null(),
                graphics_pipeline_create_info,
                None,
            )
            .unwrap()[0]
    };

    graphics_pipeline
}

fn create_main_graphics_pipeline(
    device: &ash::Device,
    pipeline_layout: vk::PipelineLayout,
    surface_format: vk::SurfaceFormatKHR,
) -> vk::Pipeline {
    let dynamic_states = &[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];

    let dynamic_state_info =
        vk::PipelineDynamicStateCreateInfo::default().dynamic_states(dynamic_states);

    let viewports = &[vk::Viewport::default()];
    let scissors = &[vk::Rect2D::default()];

    let viewport_state_info = vk::PipelineViewportStateCreateInfo::default()
        .viewports(viewports)
        .scissors(scissors);

    let vert_shader_code = fs::read(format!("{}/main.vert.spv", ASSET_PATH)).unwrap();
    let frag_shader_code = fs::read(format!("{}/main.frag.spv", ASSET_PATH)).unwrap();

    let vertex_shader = create_shader_module(device, &vert_shader_code);
    let fragment_shader = create_shader_module(device, &frag_shader_code);

    let vert_stage_info = vk::PipelineShaderStageCreateInfo::default()
        .stage(vk::ShaderStageFlags::VERTEX)
        .module(vertex_shader)
        .name(c"main");

    let frag_stage_info = vk::PipelineShaderStageCreateInfo::default()
        .stage(vk::ShaderStageFlags::FRAGMENT)
        .module(fragment_shader)
        .name(c"main");

    let shader_stages = &[vert_stage_info, frag_stage_info];

    let vertex_attribute_descriptions = MeshVertex::get_attribute_descriptions();
    let vertex_binding_description = MeshVertex::get_binding_description();

    let instance_attribute_descriptions = InstanceVertex::get_attribute_descriptions();
    let instance_binding_description = InstanceVertex::get_binding_description();

    let attribute_descriptions: Vec<vk::VertexInputAttributeDescription> =
        vertex_attribute_descriptions
            .iter()
            .chain(instance_attribute_descriptions.iter())
            .cloned()
            .collect();

    let binding_descriptions = [vertex_binding_description, instance_binding_description];

    let vertex_input_state_info = vk::PipelineVertexInputStateCreateInfo::default()
        .vertex_attribute_descriptions(&attribute_descriptions)
        .vertex_binding_descriptions(&binding_descriptions);

    let input_assembly_state_info = vk::PipelineInputAssemblyStateCreateInfo::default()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
        .primitive_restart_enable(false);

    let rasterization_info = vk::PipelineRasterizationStateCreateInfo::default()
        .depth_clamp_enable(false)
        .polygon_mode(vk::PolygonMode::FILL)
        .line_width(1.0)
        .cull_mode(vk::CullModeFlags::BACK)
        .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
        .depth_bias_enable(false);

    let multisample_info = vk::PipelineMultisampleStateCreateInfo::default()
        .sample_shading_enable(false)
        .rasterization_samples(vk::SampleCountFlags::TYPE_1);

    let color_blend_attachment_states = &[vk::PipelineColorBlendAttachmentState::default()
        .color_write_mask(vk::ColorComponentFlags::RGBA)
        .blend_enable(true)
        .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
        .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
        .color_blend_op(vk::BlendOp::ADD)
        .src_alpha_blend_factor(vk::BlendFactor::ONE)
        .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
        .alpha_blend_op(vk::BlendOp::ADD)];

    let color_blender_state_info =
        vk::PipelineColorBlendStateCreateInfo::default().attachments(color_blend_attachment_states);

    let depth_stencil_state_info = vk::PipelineDepthStencilStateCreateInfo::default()
        .depth_test_enable(true)
        .depth_write_enable(true)
        .depth_compare_op(vk::CompareOp::GREATER);

    let color_attachment_formats = [surface_format.format];
    let mut rendering_create_info = vk::PipelineRenderingCreateInfo::default()
        .color_attachment_formats(&color_attachment_formats)
        .depth_attachment_format(vk::Format::D32_SFLOAT);

    let graphics_pipeline_create_info = &[vk::GraphicsPipelineCreateInfo::default()
        .stages(shader_stages)
        .vertex_input_state(&vertex_input_state_info)
        .input_assembly_state(&input_assembly_state_info)
        .dynamic_state(&dynamic_state_info)
        .viewport_state(&viewport_state_info)
        .rasterization_state(&rasterization_info)
        .multisample_state(&multisample_info)
        .color_blend_state(&color_blender_state_info)
        .layout(pipeline_layout)
        .depth_stencil_state(&depth_stencil_state_info)
        .subpass(0)
        .push_next(&mut rendering_create_info)];

    let graphics_pipeline = unsafe {
        device
            .create_graphics_pipelines(
                vk::PipelineCache::null(),
                graphics_pipeline_create_info,
                None,
            )
            .unwrap()[0]
    };

    graphics_pipeline
}

fn create_main_transparent_graphics_pipeline(
    device: &ash::Device,
    pipeline_layout: vk::PipelineLayout,
    surface_format: vk::SurfaceFormatKHR,
) -> vk::Pipeline {
    let dynamic_states = &[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];

    let dynamic_state_info =
        vk::PipelineDynamicStateCreateInfo::default().dynamic_states(dynamic_states);

    let viewports = &[vk::Viewport::default()];
    let scissors = &[vk::Rect2D::default()];

    let viewport_state_info = vk::PipelineViewportStateCreateInfo::default()
        .viewports(viewports)
        .scissors(scissors);

    let vert_shader_code = fs::read(format!("{}/main.vert.spv", ASSET_PATH)).unwrap();
    let frag_shader_code = fs::read(format!("{}/main.frag.spv", ASSET_PATH)).unwrap();

    let vertex_shader = create_shader_module(device, &vert_shader_code);
    let fragment_shader = create_shader_module(device, &frag_shader_code);

    let vert_stage_info = vk::PipelineShaderStageCreateInfo::default()
        .stage(vk::ShaderStageFlags::VERTEX)
        .module(vertex_shader)
        .name(c"main");

    let frag_stage_info = vk::PipelineShaderStageCreateInfo::default()
        .stage(vk::ShaderStageFlags::FRAGMENT)
        .module(fragment_shader)
        .name(c"main");

    let shader_stages = &[vert_stage_info, frag_stage_info];

    let vertex_attribute_descriptions = MeshVertex::get_attribute_descriptions();
    let vertex_binding_description = MeshVertex::get_binding_description();

    let instance_attribute_descriptions = InstanceVertex::get_attribute_descriptions();
    let instance_binding_description = InstanceVertex::get_binding_description();

    let attribute_descriptions: Vec<vk::VertexInputAttributeDescription> =
        vertex_attribute_descriptions
            .iter()
            .chain(instance_attribute_descriptions.iter())
            .cloned()
            .collect();

    let binding_descriptions = [vertex_binding_description, instance_binding_description];

    let vertex_input_state_info = vk::PipelineVertexInputStateCreateInfo::default()
        .vertex_attribute_descriptions(&attribute_descriptions)
        .vertex_binding_descriptions(&binding_descriptions);

    let input_assembly_state_info = vk::PipelineInputAssemblyStateCreateInfo::default()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
        .primitive_restart_enable(false);

    let rasterization_info = vk::PipelineRasterizationStateCreateInfo::default()
        .depth_clamp_enable(false)
        .polygon_mode(vk::PolygonMode::FILL)
        .line_width(1.0)
        .cull_mode(vk::CullModeFlags::BACK)
        .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
        .depth_bias_enable(false);

    let multisample_info = vk::PipelineMultisampleStateCreateInfo::default()
        .sample_shading_enable(false)
        .rasterization_samples(vk::SampleCountFlags::TYPE_1);

    let color_blend_attachment_states = &[vk::PipelineColorBlendAttachmentState::default()
        .color_write_mask(vk::ColorComponentFlags::RGBA)
        .blend_enable(true)
        .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
        .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
        .color_blend_op(vk::BlendOp::ADD)
        .src_alpha_blend_factor(vk::BlendFactor::ONE)
        .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
        .alpha_blend_op(vk::BlendOp::ADD)];

    let color_blender_state_info =
        vk::PipelineColorBlendStateCreateInfo::default().attachments(color_blend_attachment_states);

    let depth_stencil_state_info = vk::PipelineDepthStencilStateCreateInfo::default()
        .depth_test_enable(true)
        .depth_write_enable(false)
        .depth_compare_op(vk::CompareOp::GREATER);

    let color_attachment_formats = [surface_format.format];
    let mut rendering_create_info = vk::PipelineRenderingCreateInfo::default()
        .color_attachment_formats(&color_attachment_formats)
        .depth_attachment_format(vk::Format::D32_SFLOAT);

    let graphics_pipeline_create_info = &[vk::GraphicsPipelineCreateInfo::default()
        .stages(shader_stages)
        .vertex_input_state(&vertex_input_state_info)
        .input_assembly_state(&input_assembly_state_info)
        .dynamic_state(&dynamic_state_info)
        .viewport_state(&viewport_state_info)
        .rasterization_state(&rasterization_info)
        .multisample_state(&multisample_info)
        .color_blend_state(&color_blender_state_info)
        .layout(pipeline_layout)
        .depth_stencil_state(&depth_stencil_state_info)
        .subpass(0)
        .push_next(&mut rendering_create_info)];

    let graphics_pipeline = unsafe {
        device
            .create_graphics_pipelines(
                vk::PipelineCache::null(),
                graphics_pipeline_create_info,
                None,
            )
            .unwrap()[0]
    };

    graphics_pipeline
}

fn create_sky_graphics_pipeline(
    device: &ash::Device,
    pipeline_layout: vk::PipelineLayout,
    surface_format: vk::SurfaceFormatKHR,
) -> vk::Pipeline {
    let dynamic_states = &[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];

    let dynamic_state_info =
        vk::PipelineDynamicStateCreateInfo::default().dynamic_states(dynamic_states);

    let viewports = &[vk::Viewport::default()];
    let scissors = &[vk::Rect2D::default()];

    let viewport_state_info = vk::PipelineViewportStateCreateInfo::default()
        .viewports(viewports)
        .scissors(scissors);

    let vert_shader_code = fs::read(format!("{}/skybox.vert.spv", ASSET_PATH)).unwrap();
    let frag_shader_code = fs::read(format!("{}/skybox.frag.spv", ASSET_PATH)).unwrap();

    let vertex_shader = create_shader_module(device, &vert_shader_code);
    let fragment_shader = create_shader_module(device, &frag_shader_code);

    let vert_stage_info = vk::PipelineShaderStageCreateInfo::default()
        .stage(vk::ShaderStageFlags::VERTEX)
        .module(vertex_shader)
        .name(c"main");

    let frag_stage_info = vk::PipelineShaderStageCreateInfo::default()
        .stage(vk::ShaderStageFlags::FRAGMENT)
        .module(fragment_shader)
        .name(c"main");

    let shader_stages = &[vert_stage_info, frag_stage_info];

    let vertex_attribute_descriptions = MeshVertex::get_attribute_descriptions();
    let vertex_binding_descriptions = [MeshVertex::get_binding_description()];

    let vertex_input_state_info = vk::PipelineVertexInputStateCreateInfo::default()
        .vertex_attribute_descriptions(&vertex_attribute_descriptions)
        .vertex_binding_descriptions(&vertex_binding_descriptions);

    let input_assembly_state_info = vk::PipelineInputAssemblyStateCreateInfo::default()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
        .primitive_restart_enable(false);

    let rasterization_info = vk::PipelineRasterizationStateCreateInfo::default()
        .depth_clamp_enable(false)
        .polygon_mode(vk::PolygonMode::FILL)
        .line_width(1.0)
        .cull_mode(vk::CullModeFlags::FRONT)
        .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
        .depth_bias_enable(false);

    let multisample_info = vk::PipelineMultisampleStateCreateInfo::default()
        .sample_shading_enable(false)
        .rasterization_samples(vk::SampleCountFlags::TYPE_1);

    let color_blend_attachment_states = &[vk::PipelineColorBlendAttachmentState::default()
        .blend_enable(false)
        .color_write_mask(vk::ColorComponentFlags::RGBA)
        .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
        .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
        .color_blend_op(vk::BlendOp::ADD)
        .src_alpha_blend_factor(vk::BlendFactor::ONE)
        .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
        .alpha_blend_op(vk::BlendOp::ADD)];

    let color_blender_state_info =
        vk::PipelineColorBlendStateCreateInfo::default().attachments(color_blend_attachment_states);

    let depth_stencil_state_info = vk::PipelineDepthStencilStateCreateInfo::default()
        .depth_test_enable(false)
        .depth_write_enable(false)
        .depth_compare_op(vk::CompareOp::GREATER);

    let color_attachment_formats = [surface_format.format];
    let mut rendering_create_info = vk::PipelineRenderingCreateInfo::default()
        .color_attachment_formats(&color_attachment_formats)
        .depth_attachment_format(vk::Format::D32_SFLOAT);

    let graphics_pipeline_create_info = &[vk::GraphicsPipelineCreateInfo::default()
        .stages(shader_stages)
        .vertex_input_state(&vertex_input_state_info)
        .input_assembly_state(&input_assembly_state_info)
        .dynamic_state(&dynamic_state_info)
        .viewport_state(&viewport_state_info)
        .rasterization_state(&rasterization_info)
        .multisample_state(&multisample_info)
        .color_blend_state(&color_blender_state_info)
        .layout(pipeline_layout)
        .depth_stencil_state(&depth_stencil_state_info)
        .subpass(0)
        .push_next(&mut rendering_create_info)];

    let graphics_pipeline = unsafe {
        device
            .create_graphics_pipelines(
                vk::PipelineCache::null(),
                graphics_pipeline_create_info,
                None,
            )
            .unwrap()[0]
    };

    graphics_pipeline
}

fn create_shadow_graphics_pipeline(
    device: &ash::Device,
    pipeline_layout: vk::PipelineLayout,
) -> vk::Pipeline {
    let dynamic_states = &[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];

    let dynamic_state_info =
        vk::PipelineDynamicStateCreateInfo::default().dynamic_states(dynamic_states);

    let viewports = &[vk::Viewport::default()];
    let scissors = &[vk::Rect2D::default()];

    let viewport_state_info = vk::PipelineViewportStateCreateInfo::default()
        .viewports(viewports)
        .scissors(scissors);

    let vert_shader_code = fs::read(format!("{}/shadow.vert.spv", ASSET_PATH)).unwrap();
    let frag_shader_code = fs::read(format!("{}/shadow.frag.spv", ASSET_PATH)).unwrap();

    let vertex_shader = create_shader_module(device, &vert_shader_code);
    let fragment_shader = create_shader_module(device, &frag_shader_code);

    let vert_stage_info = vk::PipelineShaderStageCreateInfo::default()
        .stage(vk::ShaderStageFlags::VERTEX)
        .module(vertex_shader)
        .name(c"main");

    let frag_stage_info = vk::PipelineShaderStageCreateInfo::default()
        .stage(vk::ShaderStageFlags::FRAGMENT)
        .module(fragment_shader)
        .name(c"main");

    let shader_stages = &[vert_stage_info, frag_stage_info];

    let vertex_attribute_descriptions = MeshVertex::get_attribute_descriptions();
    let vertex_binding_description = MeshVertex::get_binding_description();

    let instance_attribute_descriptions = InstanceVertex::get_attribute_descriptions();
    let instance_binding_description = InstanceVertex::get_binding_description();

    let attribute_descriptions: Vec<vk::VertexInputAttributeDescription> =
        vertex_attribute_descriptions
            .iter()
            .chain(instance_attribute_descriptions.iter())
            .cloned()
            .collect();

    let binding_descriptions = [vertex_binding_description, instance_binding_description];

    let vertex_input_state_info = vk::PipelineVertexInputStateCreateInfo::default()
        .vertex_attribute_descriptions(&attribute_descriptions)
        .vertex_binding_descriptions(&binding_descriptions);

    let input_assembly_state_info = vk::PipelineInputAssemblyStateCreateInfo::default()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
        .primitive_restart_enable(false);

    let rasterization_info = vk::PipelineRasterizationStateCreateInfo::default()
        .depth_clamp_enable(false)
        .polygon_mode(vk::PolygonMode::FILL)
        .line_width(1.0)
        .cull_mode(vk::CullModeFlags::BACK)
        .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
        .depth_bias_enable(false);

    let multisample_info = vk::PipelineMultisampleStateCreateInfo::default()
        .sample_shading_enable(false)
        .rasterization_samples(vk::SampleCountFlags::TYPE_1);

    let depth_stencil_state_info = vk::PipelineDepthStencilStateCreateInfo::default()
        .depth_test_enable(true)
        .depth_write_enable(true)
        .depth_compare_op(vk::CompareOp::GREATER);

    let mut rendering_create_info =
        vk::PipelineRenderingCreateInfo::default().depth_attachment_format(vk::Format::D32_SFLOAT);

    let graphics_pipeline_create_info = &[vk::GraphicsPipelineCreateInfo::default()
        .stages(shader_stages)
        .vertex_input_state(&vertex_input_state_info)
        .input_assembly_state(&input_assembly_state_info)
        .dynamic_state(&dynamic_state_info)
        .viewport_state(&viewport_state_info)
        .rasterization_state(&rasterization_info)
        .multisample_state(&multisample_info)
        .layout(pipeline_layout)
        .depth_stencil_state(&depth_stencil_state_info)
        .subpass(0)
        .push_next(&mut rendering_create_info)];

    let graphics_pipeline = unsafe {
        device
            .create_graphics_pipelines(
                vk::PipelineCache::null(),
                graphics_pipeline_create_info,
                None,
            )
            .unwrap()[0]
    };

    graphics_pipeline
}
