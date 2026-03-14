use std::{fs, mem};

use ash::{prelude::VkResult, vk};
use glam::Mat4;
use vk_mem::Alloc;

use crate::{
    assets::ASSET_PATH,
    renderer::{
        mesh::{CameraUniform2d, Vertex2d},
        pipelines::VulkanPipelineBuilder,
        renderer::TextureDescriptors,
        vkutils,
    },
    scene::RenderScene,
    shapes::Rect,
    text::GlyphAtlas,
};

const MAX_VERTICES_2D: usize = 50000;

pub struct Resources {
    pub camera_descriptor_set: vk::DescriptorSet,
    pub camera_uniform_buffer: vkutils::AllocatedBuffer,
    pub vertex_buffer: vkutils::AllocatedBuffer,
    pub index_buffer: vkutils::AllocatedBuffer,
    pub glyph_atlas_image: vkutils::AllocatedImage,
    pub dirty_region: Option<Rect>,
    pub vertex_count: u32,
}

impl Resources {
    pub fn new(
        device: &ash::Device,
        allocator: &vk_mem::Allocator,
        descriptor_pool: vk::DescriptorPool,
        frame_layout_2d: vk::DescriptorSetLayout,
        glyph_atlas: &GlyphAtlas,
    ) -> VkResult<Self> {
        let layouts = [frame_layout_2d];
        let descriptor_set_alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&layouts);

        let descriptor_sets =
            unsafe { device.allocate_descriptor_sets(&descriptor_set_alloc_info)? };

        let camera2d_descriptor_set = descriptor_sets[0];

        let global2d_uniform_buffer_info = vk::BufferCreateInfo::default()
            .usage(vk::BufferUsageFlags::UNIFORM_BUFFER)
            .size(mem::size_of::<CameraUniform2d>() as u64);

        let uniform_alloc_info = vk_mem::AllocationCreateInfo {
            flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE
                | vk_mem::AllocationCreateFlags::MAPPED,
            usage: vk_mem::MemoryUsage::AutoPreferHost,

            ..Default::default()
        };

        let camera_uniform_buffer =
            unsafe { allocator.create_buffer(&global2d_uniform_buffer_info, &uniform_alloc_info)? };

        let camera_descriptor_buffer_info = [vk::DescriptorBufferInfo::default()
            .offset(0)
            .range(mem::size_of::<CameraUniform2d>() as u64)
            .buffer(camera_uniform_buffer.0)];

        let descriptor_write = [vk::WriteDescriptorSet::default()
            .dst_set(camera2d_descriptor_set)
            .dst_binding(0)
            .dst_array_element(0)
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .buffer_info(&camera_descriptor_buffer_info)];

        unsafe { device.update_descriptor_sets(&descriptor_write, &[]) };

        let (vertex_buffer, index_buffer) = Self::create_vertex_buffer(allocator)?;

        let glyph_atlas_image =
            Self::create_glyph_atlas_image(allocator, glyph_atlas.width, glyph_atlas.height)?;

        Ok(Resources {
            camera_uniform_buffer,
            camera_descriptor_set: camera2d_descriptor_set,
            vertex_buffer,
            index_buffer,
            glyph_atlas_image,
            vertex_count: 0,
            dirty_region: Some(Rect {
                x: 0,
                y: 0,
                width: glyph_atlas.width,
                height: glyph_atlas.height,
            }),
        })
    }

    pub fn destroy(&mut self, _device: &ash::Device, allocator: &vk_mem::Allocator) {
        vkutils::destroy_allocated_buffer(allocator, &mut self.camera_uniform_buffer);
        vkutils::destroy_allocated_buffer(allocator, &mut self.vertex_buffer);
        vkutils::destroy_allocated_buffer(allocator, &mut self.index_buffer);
        vkutils::destroy_allocated_image(allocator, &mut self.glyph_atlas_image);
    }

    pub fn update_buffers(&mut self, allocator: &vk_mem::Allocator, scene: &RenderScene) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for ui_frame in scene.ui.iter() {
            ui_frame.push_verts(&mut vertices, &mut indices);
        }

        let vertex_alloc_info = allocator.get_allocation_info(&self.vertex_buffer.1);
        let index_alloc_info = allocator.get_allocation_info(&self.index_buffer.1);

        unsafe {
            std::ptr::copy_nonoverlapping(
                vertices.as_ptr(),
                vertex_alloc_info.mapped_data.cast(),
                vertices.len().min(MAX_VERTICES_2D),
            );

            std::ptr::copy_nonoverlapping(
                indices.as_ptr(),
                index_alloc_info.mapped_data.cast(),
                indices.len().min(MAX_VERTICES_2D),
            );
        };

        self.vertex_count = indices.len() as u32;
    }

    pub fn draw_scene(
        &mut self,
        device: &ash::Device,
        command_buffer: vk::CommandBuffer,
        scene_pipeline: vk::Pipeline,
        pipeline_layout: vk::PipelineLayout,
        textures: &[TextureDescriptors],
    ) {
        if self.vertex_count == 0 {
            return;
        }

        unsafe {
            let descriptor_sets = [self.camera_descriptor_set, textures[1].descriptor_set];

            device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline_layout,
                0,
                &descriptor_sets,
                &[],
            );

            device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                scene_pipeline,
            );

            device.cmd_bind_vertex_buffers(command_buffer, 0, &[self.vertex_buffer.0], &[0]);

            device.cmd_bind_index_buffer(
                command_buffer,
                self.index_buffer.0,
                0,
                vk::IndexType::UINT16,
            );

            device.cmd_draw_indexed(command_buffer, self.vertex_count, 1, 0, 0, 0);
        }
    }

    fn create_glyph_atlas_image(
        allocator: &vk_mem::Allocator,
        width: u32,
        height: u32,
    ) -> VkResult<vkutils::AllocatedImage> {
        let image_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .extent(vk::Extent3D {
                width: width,
                height: height,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .format(vk::Format::R8_UNORM)
            .tiling(vk::ImageTiling::LINEAR)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .usage(vk::ImageUsageFlags::SAMPLED)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .samples(vk::SampleCountFlags::TYPE_1)
            .flags(vk::ImageCreateFlags::empty());

        let image_alloc_info = vk_mem::AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::AutoPreferDevice,
            ..Default::default()
        };

        let image = unsafe { allocator.create_image(&image_info, &image_alloc_info)? };

        Ok(image)
    }

    fn create_vertex_buffer(
        allocator: &vk_mem::Allocator,
    ) -> VkResult<(vkutils::AllocatedBuffer, vkutils::AllocatedBuffer)> {
        let vertex_buffer_info = vk::BufferCreateInfo::default()
            .size((mem::size_of::<u16>() * MAX_VERTICES_2D) as u64)
            .usage(vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST);

        let index_buffer_info = vk::BufferCreateInfo::default()
            .size((mem::size_of::<Vertex2d>() * MAX_VERTICES_2D) as u64)
            .usage(vk::BufferUsageFlags::INDEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST);

        let alloc_info = vk_mem::AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::AutoPreferHost,
            flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE
                | vk_mem::AllocationCreateFlags::MAPPED,
            ..Default::default()
        };

        let vertex_buffer = unsafe { allocator.create_buffer(&vertex_buffer_info, &alloc_info)? };
        let index_buffer = unsafe { allocator.create_buffer(&index_buffer_info, &alloc_info)? };

        Ok((vertex_buffer, index_buffer))
    }

    pub fn update_uniform_buffers(
        &self,
        allocator: &vk_mem::Allocator,
        window_extent: vk::Extent2D,
    ) {
        let view = Mat4::IDENTITY;
        let proj = Mat4::orthographic_rh(
            0.0,
            window_extent.width as f32,
            0.0,
            window_extent.height as f32,
            0.0,
            1.0,
        );

        let camera_data = CameraUniform2d { proj, view };

        let camera_uniform_buffer_alloc_info =
            allocator.get_allocation_info(&self.camera_uniform_buffer.1);

        unsafe {
            std::ptr::copy_nonoverlapping(
                &camera_data,
                camera_uniform_buffer_alloc_info.mapped_data.cast(),
                1,
            );
        }
    }
}

pub fn create_scene2_pipeline(
    device: &ash::Device,
    pipeline_layout: vk::PipelineLayout,
    surface_format: vk::SurfaceFormatKHR,
) -> VkResult<vk::Pipeline> {
    let vert_shader_code = fs::read(format!("{}/main2d.vert.spv", ASSET_PATH)).unwrap();
    let frag_shader_code = fs::read(format!("{}/main2d.frag.spv", ASSET_PATH)).unwrap();
    let vertex_shader = vkutils::create_shader_module(device, &vert_shader_code).unwrap();
    let fragment_shader = vkutils::create_shader_module(device, &frag_shader_code).unwrap();

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

    pipeline_builder.build(device)
}
