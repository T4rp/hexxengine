use std::{fs, io::Cursor};

use ash::{
    prelude::VkResult,
    vk::{self, PipelineLayout},
};

use crate::{
    assets::ASSET_PATH,
    renderer::{
        mesh::{InstanceVertex, MeshVertex, Vertex2d},
        scene3d::Scene3dPipelineObjects,
        vkutils::create_shader_module,
    },
};

pub struct RendererPipelineObjects {
    pub main_2d_graphics_pipeline: vk::Pipeline,
    pub main_graphics_pipeline: vk::Pipeline,
    pub shadow_graphics_pipeline: vk::Pipeline,
    pub main_transparent_graphics_pipeline: vk::Pipeline,
    pub skybox_graphics_pipeline: vk::Pipeline,
}

impl RendererPipelineObjects {
    pub fn new(
        device: &ash::Device,
        pipeline_layout_3d: vk::PipelineLayout,
        pipeline_layout_2d: vk::PipelineLayout,
        surface_format: vk::SurfaceFormatKHR,
    ) -> Self {
        let pipelines_3d =
            Scene3dPipelineObjects::new(device, pipeline_layout_3d, surface_format.format).unwrap();

        let main_graphics_pipeline = pipelines_3d.opaque_pipeline;
        let main_transparent_graphics_pipeline = pipelines_3d.transparent_pipeline;
        let shadow_graphics_pipeline = pipelines_3d.shadow_pipeline;

        let skybox_graphics_pipeline =
            create_sky_graphics_pipeline(device, pipeline_layout_3d, surface_format);

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
    let vert_shader_code = fs::read(format!("{}/main2d.vert.spv", ASSET_PATH)).unwrap();
    let frag_shader_code = fs::read(format!("{}/main2d.frag.spv", ASSET_PATH)).unwrap();
    let vertex_shader = create_shader_module(device, &vert_shader_code).unwrap();
    let fragment_shader = create_shader_module(device, &frag_shader_code).unwrap();

    let shader_stages = [
        vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(vertex_shader)
            .name(c"main"),
        vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(fragment_shader)
            .name(c"main"),
    ];

    let vertex_attribute_descriptions = Vertex2d::get_attribute_descriptions();
    let vertex_binding_description = Vertex2d::get_binding_descriptions();

    let pipeline_builder = VulkanPipelineBuilder::default()
        .pipeline_layout(pipeline_layout)
        .shader_stages(&shader_stages)
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
        .color_attachment_format(surface_format.format)
        .depth_attachment_format(vk::Format::D32_SFLOAT);

    pipeline_builder.build(device).unwrap()
}

fn create_sky_graphics_pipeline(
    device: &ash::Device,
    pipeline_layout: vk::PipelineLayout,
    surface_format: vk::SurfaceFormatKHR,
) -> vk::Pipeline {
    let vert_shader_code = fs::read(format!("{}/skybox.vert.spv", ASSET_PATH)).unwrap();
    let frag_shader_code = fs::read(format!("{}/skybox.frag.spv", ASSET_PATH)).unwrap();

    let vertex_shader = create_shader_module(device, &vert_shader_code).unwrap();
    let fragment_shader = create_shader_module(device, &frag_shader_code).unwrap();

    let shader_stages = [
        vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(vertex_shader)
            .name(c"main"),
        vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(fragment_shader)
            .name(c"main"),
    ];

    let vertex_attributes = MeshVertex::get_attribute_descriptions();
    let vertex_bindings = [MeshVertex::get_binding_description()];

    let pipeline_builder = VulkanPipelineBuilder::default()
        .pipeline_layout(pipeline_layout)
        .shader_stages(&shader_stages)
        .vertex_input_state(
            vk::PipelineVertexInputStateCreateInfo::default()
                .vertex_attribute_descriptions(&vertex_attributes)
                .vertex_binding_descriptions(&vertex_bindings),
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
                .cull_mode(vk::CullModeFlags::FRONT)
                .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
                .depth_bias_enable(false),
        )
        .multisampling(
            vk::PipelineMultisampleStateCreateInfo::default()
                .sample_shading_enable(false)
                .rasterization_samples(vk::SampleCountFlags::TYPE_1),
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
        )
        .color_attachment_format(surface_format.format)
        .depth_attachment_format(vk::Format::D32_SFLOAT);

    pipeline_builder.build(device).unwrap()
}

#[derive(Debug, Clone)]
pub struct VulkanPipelineBuilder<'a> {
    shader_stages: Option<&'a [vk::PipelineShaderStageCreateInfo<'a>]>,
    vertex_input: Option<vk::PipelineVertexInputStateCreateInfo<'a>>,
    input_assembly_state: Option<vk::PipelineInputAssemblyStateCreateInfo<'a>>,
    viewport: Option<vk::Viewport>,
    scissor: Option<vk::Rect2D>,
    rasterizer_state: Option<vk::PipelineRasterizationStateCreateInfo<'a>>,
    colorblend_state: Option<vk::PipelineColorBlendAttachmentState>,
    multisampling_state: Option<vk::PipelineMultisampleStateCreateInfo<'a>>,
    depth_stencil_state: Option<vk::PipelineDepthStencilStateCreateInfo<'a>>,
    pipeline_layout: Option<vk::PipelineLayout>,
    color_attachment_format: Option<vk::Format>,
    depth_attachment_format: Option<vk::Format>,
}

impl<'a> VulkanPipelineBuilder<'a> {
    pub fn build(&self, device: &ash::Device) -> VkResult<vk::Pipeline> {
        let shader_stages = self.shader_stages.unwrap();
        let vertex_input_state_info = self.vertex_input.unwrap();
        let input_assembly_state_info = self.input_assembly_state.unwrap();
        let rasterization_info = self.rasterizer_state.unwrap();
        let multisample_info = self.multisampling_state.unwrap();
        let pipeline_layout = self.pipeline_layout.unwrap();
        let depth_stencil_state_info = self.depth_stencil_state.unwrap();
        let depth_attachment_format = self.depth_attachment_format.unwrap();

        let color_attachment_formats: &[vk::Format] = match self.color_attachment_format {
            Some(format) => &[format],
            None => &[],
        };

        let colorblend_state_attachments: &[vk::PipelineColorBlendAttachmentState] =
            match self.colorblend_state {
                Some(attachment_state) => &[attachment_state],
                None => &[],
            };

        let colorblend_state_info = vk::PipelineColorBlendStateCreateInfo::default()
            .attachments(&colorblend_state_attachments);

        let dynamic_state_info = vk::PipelineDynamicStateCreateInfo::default()
            .dynamic_states(&[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR]);

        let viewports = [self.viewport.unwrap()];
        let scissors = [self.scissor.unwrap()];
        let viewport_state_info = vk::PipelineViewportStateCreateInfo::default()
            .viewports(&viewports)
            .scissors(&scissors);

        let mut rendering_create_info = vk::PipelineRenderingCreateInfo::default()
            .color_attachment_formats(color_attachment_formats)
            .depth_attachment_format(depth_attachment_format);

        let graphics_pipeline_create_info = &[vk::GraphicsPipelineCreateInfo::default()
            .stages(&shader_stages)
            .vertex_input_state(&vertex_input_state_info)
            .input_assembly_state(&input_assembly_state_info)
            .dynamic_state(&dynamic_state_info)
            .viewport_state(&viewport_state_info)
            .rasterization_state(&rasterization_info)
            .multisample_state(&multisample_info)
            .color_blend_state(&colorblend_state_info)
            .layout(pipeline_layout)
            .depth_stencil_state(&depth_stencil_state_info)
            .subpass(0)
            .push_next(&mut rendering_create_info)];

        let graphics_pipelines = unsafe {
            device.create_graphics_pipelines(
                vk::PipelineCache::null(),
                graphics_pipeline_create_info,
                None,
            )
        }
        .map_err(|(_pipelines, vk_result)| vk_result)?;

        Ok(graphics_pipelines[0])
    }

    pub fn viewport(mut self, viewport: vk::Viewport) -> Self {
        self.viewport = Some(viewport);
        self
    }

    pub fn scissor(mut self, scissor: vk::Rect2D) -> Self {
        self.scissor = Some(scissor);
        self
    }

    pub fn shader_stages(
        mut self,
        shader_stage_info: &'a [vk::PipelineShaderStageCreateInfo<'a>],
    ) -> VulkanPipelineBuilder<'a> {
        self.shader_stages = Some(shader_stage_info);
        self
    }

    pub fn vertex_input_state(
        mut self,
        vertex_input_state: vk::PipelineVertexInputStateCreateInfo<'a>,
    ) -> VulkanPipelineBuilder<'a> {
        self.vertex_input = Some(vertex_input_state);
        self
    }

    pub fn input_assembly_state(
        mut self,
        input_assembly_info: vk::PipelineInputAssemblyStateCreateInfo<'a>,
    ) -> VulkanPipelineBuilder<'a> {
        self.input_assembly_state = Some(input_assembly_info);
        self
    }

    pub fn rasterizer(mut self, rasterizer: vk::PipelineRasterizationStateCreateInfo<'a>) -> Self {
        self.rasterizer_state = Some(rasterizer);
        self
    }

    pub fn multisampling(
        mut self,
        multisampling: vk::PipelineMultisampleStateCreateInfo<'a>,
    ) -> Self {
        self.multisampling_state = Some(multisampling);
        self
    }

    pub fn colorblend_state(
        mut self,
        colorblend_state: vk::PipelineColorBlendAttachmentState,
    ) -> Self {
        self.colorblend_state = Some(colorblend_state);
        self
    }

    pub fn depth_stencil_state(
        mut self,
        depth_stencil_state: vk::PipelineDepthStencilStateCreateInfo<'a>,
    ) -> Self {
        self.depth_stencil_state = Some(depth_stencil_state);
        self
    }

    pub fn pipeline_layout(mut self, layout: vk::PipelineLayout) -> Self {
        self.pipeline_layout = Some(layout);
        self
    }

    pub fn color_attachment_format(mut self, format: vk::Format) -> Self {
        self.color_attachment_format = Some(format);
        self
    }

    pub fn no_color_attachments(mut self) -> Self {
        self.color_attachment_format = None;
        self.colorblend_state = None;
        self
    }

    pub fn depth_attachment_format(mut self, format: vk::Format) -> Self {
        self.depth_attachment_format = Some(format);
        self
    }
}

impl<'a> Default for VulkanPipelineBuilder<'a> {
    fn default() -> Self {
        Self {
            viewport: Some(vk::Viewport::default()),
            scissor: Some(vk::Rect2D::default()),
            shader_stages: None,
            vertex_input: None,
            input_assembly_state: None,
            rasterizer_state: None,
            multisampling_state: None,
            pipeline_layout: None,
            colorblend_state: None,
            depth_stencil_state: None,
            depth_attachment_format: None,
            color_attachment_format: None,
        }
    }
}
