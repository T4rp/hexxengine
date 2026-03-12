use std::{fs, mem};

use ash::{prelude::VkResult, vk};
use vk_mem::Alloc;

use crate::{
    assets::ASSET_PATH,
    renderer::{
        mesh::{CameraUniform3d, InstanceVertex, MeshVertex, Scene3dUniform},
        pipelines::VulkanPipelineBuilder,
        textures::Texture,
        vkutils,
    },
};

const SHADOW_MAP_RESOLUTION: u32 = 2048;
const MAX_INSTANCE_COUNT: usize = 10000;

pub struct Scene3dResources {
    pub main_pass_descriptor_set: vk::DescriptorSet,
    pub shadow_pass_descriptor_set: vk::DescriptorSet,

    pub shadow_map_sampler: vk::Sampler,
    pub skybox_outdated: bool,

    pub camera_uniform_buffer: (vk::Buffer, vk_mem::Allocation),
    pub scene_uniform_buffer: (vk::Buffer, vk_mem::Allocation),

    pub instance_buffer: (vk::Buffer, vk_mem::Allocation),

    pub depth_image_view: vk::ImageView,
    pub depth_image: vkutils::AllocatedImage,
    pub extents_outdated: bool,

    pub shadow_map_image: vkutils::AllocatedImage,
    pub shadow_map_image_view: vk::ImageView,
}

impl Scene3dResources {
    pub fn new(
        device: &ash::Device,
        allocator: &vk_mem::Allocator,
        command_pool: vk::CommandPool,
        queue: vk::Queue,
        queue_family_index: u32,
        descriptor_pool: vk::DescriptorPool,
        scene_descriptor_layout: vk::DescriptorSetLayout,
        window_extent: vk::Extent2D,
    ) -> VkResult<Self> {
        let command_buffer = vkutils::allocate_command_buffer(
            device,
            command_pool,
            vk::CommandBufferLevel::PRIMARY,
        )?;

        let depth_image = Self::create_depth_image(
            device,
            allocator,
            queue,
            command_pool,
            window_extent.width,
            window_extent.height,
            vk::ImageUsageFlags::empty(),
            vk::MemoryPropertyFlags::LAZILY_ALLOCATED,
        )?;
        let depth_image_view = Self::create_depth_image_view(device, &depth_image)?;

        let shadow_map_image = Self::create_depth_image(
            device,
            allocator,
            queue,
            command_pool,
            SHADOW_MAP_RESOLUTION,
            SHADOW_MAP_RESOLUTION,
            vk::ImageUsageFlags::SAMPLED,
            vk::MemoryPropertyFlags::empty(),
        )?;
        let shadow_map_image_view = Self::create_depth_image_view(device, &shadow_map_image)?;
        let shadow_map_sampler = Self::create_shadow_map_sampler(device)?;

        let camera_uniform_buffer = Self::create_camera_uniform_buffer(allocator)?;
        let scene_uniform_buffer = Self::create_scene_uniform_buffer(allocator)?;

        let (main_pass_descriptor_set, shadow_pass_descriptor_set) = Self::create_descriptor_sets(
            device,
            descriptor_pool,
            scene_descriptor_layout,
            camera_uniform_buffer,
            scene_uniform_buffer,
            shadow_map_image_view,
            shadow_map_sampler,
        )?;

        let instance_buffer = Self::create_instance_buffer(allocator)?;

        Ok(Self {
            main_pass_descriptor_set,
            shadow_pass_descriptor_set,
            shadow_map_sampler,
            skybox_outdated: true,
            camera_uniform_buffer,
            scene_uniform_buffer,
            instance_buffer,
            depth_image_view,
            depth_image,
            extents_outdated: false,
            shadow_map_image,
            shadow_map_image_view,
        })
    }

    pub fn target_resized(
        &mut self,
        device: &ash::Device,
        allocator: &vk_mem::Allocator,
        command_pool: vk::CommandPool,
        queue: vk::Queue,
        window_extent: vk::Extent2D,
    ) -> VkResult<()> {
        vkutils::destroy_allocated_image(allocator, &mut self.depth_image);
        unsafe {
            device.destroy_image_view(self.depth_image_view, None);
        }

        let depth_image = Self::create_depth_image(
            device,
            allocator,
            queue,
            command_pool,
            window_extent.width,
            window_extent.height,
            vk::ImageUsageFlags::empty(),
            vk::MemoryPropertyFlags::LAZILY_ALLOCATED,
        )?;
        let depth_image_view = Self::create_depth_image_view(device, &depth_image)?;

        self.depth_image = depth_image;
        self.depth_image_view = depth_image_view;

        todo!()
    }

    pub fn update_skybox(&mut self, device: &ash::Device, skybox_texture: &Texture) {
        let skybox_image_info = [vk::DescriptorImageInfo::default()
            .image_view(skybox_texture.image_view)
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .sampler(
                skybox_texture
                    .sampler
                    .expect("No sampler in skybox texture"),
            )];

        let descriptor_write = [vk::WriteDescriptorSet::default()
            .dst_set(self.main_pass_descriptor_set)
            .dst_binding(3)
            .dst_array_element(0)
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(&skybox_image_info)];

        unsafe { device.update_descriptor_sets(&descriptor_write, &[]) };
    }

    fn create_instance_buffer(allocator: &vk_mem::Allocator) -> VkResult<vkutils::AllocatedBuffer> {
        let instance_buffer_info = vk::BufferCreateInfo::default()
            .size((mem::size_of::<InstanceVertex>() * MAX_INSTANCE_COUNT) as u64)
            .usage(vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST);

        let alloc_info = vk_mem::AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::AutoPreferHost,
            flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE
                | vk_mem::AllocationCreateFlags::MAPPED,
            ..Default::default()
        };

        unsafe { allocator.create_buffer(&instance_buffer_info, &alloc_info) }
    }

    fn create_descriptor_sets(
        device: &ash::Device,
        descriptor_pool: vk::DescriptorPool,
        scene_descriptor_layout: vk::DescriptorSetLayout,
        camera_buffer: vkutils::AllocatedBuffer,
        scene_buffer: vkutils::AllocatedBuffer,
        shadow_map_view: vk::ImageView,
        shadow_map_sampler: vk::Sampler,
    ) -> VkResult<(vk::DescriptorSet, vk::DescriptorSet)> {
        // one for shadow map pass, one for main scene pass
        let layouts = [scene_descriptor_layout; 2];
        let descriptor_set_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&layouts);

        let descriptor_sets = unsafe { device.allocate_descriptor_sets(&descriptor_set_info)? };
        let main_pass_descriptor_set = descriptor_sets[0];
        let shadow_pass_descriptor_set = descriptor_sets[1];

        let shadow_map_image_info = [vk::DescriptorImageInfo::default()
            .image_view(shadow_map_view)
            .image_layout(vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL)
            .sampler(shadow_map_sampler)];

        let camera_buffer_info = [vk::DescriptorBufferInfo::default()
            .offset(0)
            .range(mem::size_of::<CameraUniform3d>() as u64)
            .buffer(camera_buffer.0)];

        let scene_buffer_info = [vk::DescriptorBufferInfo::default()
            .offset(0)
            .range(mem::size_of::<Scene3dUniform>() as u64)
            .buffer(scene_buffer.0)];

        let descriptor_write = [
            vk::WriteDescriptorSet::default()
                .dst_set(shadow_pass_descriptor_set)
                .dst_binding(0)
                .dst_array_element(0)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(&camera_buffer_info),
            vk::WriteDescriptorSet::default()
                .dst_set(shadow_pass_descriptor_set)
                .dst_binding(1)
                .dst_array_element(0)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(&scene_buffer_info),
            vk::WriteDescriptorSet::default()
                .dst_set(main_pass_descriptor_set)
                .dst_binding(0)
                .dst_array_element(0)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(&camera_buffer_info),
            vk::WriteDescriptorSet::default()
                .dst_set(main_pass_descriptor_set)
                .dst_binding(1)
                .dst_array_element(0)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(&scene_buffer_info),
            vk::WriteDescriptorSet::default()
                .dst_set(main_pass_descriptor_set)
                .dst_binding(2)
                .dst_array_element(0)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(&shadow_map_image_info),
        ];

        unsafe { device.update_descriptor_sets(&descriptor_write, &[]) };

        Ok((main_pass_descriptor_set, shadow_pass_descriptor_set))
    }

    fn create_camera_uniform_buffer(
        allocator: &vk_mem::Allocator,
    ) -> VkResult<vkutils::AllocatedBuffer> {
        vkutils::create_uniform_buffer::<CameraUniform3d>(allocator)
    }

    fn create_scene_uniform_buffer(
        allocator: &vk_mem::Allocator,
    ) -> VkResult<vkutils::AllocatedBuffer> {
        vkutils::create_uniform_buffer::<Scene3dUniform>(allocator)
    }

    fn create_shadow_map_sampler(device: &ash::Device) -> VkResult<vk::Sampler> {
        let shadow_map_sampler_info = vk::SamplerCreateInfo::default()
            .mag_filter(vk::Filter::NEAREST)
            .min_filter(vk::Filter::NEAREST)
            // .compare_enable(false)
            // .compare_op(vk::CompareOp::GREATER)
            .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_BORDER)
            .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_BORDER)
            .border_color(vk::BorderColor::FLOAT_OPAQUE_BLACK);

        unsafe { device.create_sampler(&shadow_map_sampler_info, None) }
    }

    fn create_depth_image_view(
        device: &ash::Device,
        depth_image: &vkutils::AllocatedImage,
    ) -> VkResult<vk::ImageView> {
        let image_view_info = vk::ImageViewCreateInfo::default()
            .image(depth_image.0)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(vk::Format::D32_SFLOAT)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::DEPTH,
                base_mip_level: 0,
                level_count: vk::REMAINING_MIP_LEVELS,
                base_array_layer: 0,
                layer_count: vk::REMAINING_ARRAY_LAYERS,
            });

        unsafe { device.create_image_view(&image_view_info, None) }
    }

    fn create_shadow_map_image_view(
        device: &ash::Device,
        depth_image: &vkutils::AllocatedImage,
    ) -> VkResult<vk::ImageView> {
        let image_view_info = vk::ImageViewCreateInfo::default()
            .image(depth_image.0)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(vk::Format::D32_SFLOAT)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::DEPTH,
                base_mip_level: 0,
                level_count: vk::REMAINING_MIP_LEVELS,
                base_array_layer: 0,
                layer_count: vk::REMAINING_ARRAY_LAYERS,
            });

        unsafe { device.create_image_view(&image_view_info, None) }
    }

    fn create_depth_image(
        device: &ash::Device,
        allocator: &vk_mem::Allocator,
        queue: vk::Queue,
        command_pool: vk::CommandPool,
        width: u32,
        height: u32,
        usage_flags: vk::ImageUsageFlags,
        preferred_allocation_flags: vk::MemoryPropertyFlags,
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
            .format(vk::Format::D32_SFLOAT)
            .tiling(vk::ImageTiling::OPTIMAL)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT | usage_flags)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .samples(vk::SampleCountFlags::TYPE_1);

        let image_alloc_info = vk_mem::AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::AutoPreferDevice,
            preferred_flags: preferred_allocation_flags,
            ..Default::default()
        };

        let depth_image = unsafe { allocator.create_image(&image_info, &image_alloc_info)? };

        let command_buffers = vkutils::allocate_command_buffers(
            device,
            command_pool,
            1,
            vk::CommandBufferLevel::PRIMARY,
        )?;

        let command_buffer_being_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe { device.begin_command_buffer(command_buffers[0], &command_buffer_being_info)? };

        vkutils::transition_image(
            device,
            command_buffers[0],
            depth_image.0,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
            vk::ImageAspectFlags::DEPTH,
        );

        let submit_info = [vk::SubmitInfo::default().command_buffers(&command_buffers)];

        unsafe {
            device.end_command_buffer(command_buffers[0])?;
            device.queue_submit(queue, &submit_info, vk::Fence::null())?;
            device.queue_wait_idle(queue)?;
            device.free_command_buffers(command_pool, &command_buffers);
        }

        Ok(depth_image)
    }
}

pub struct Scene3dPipelineObjects {
    pub opaque_pipeline: vk::Pipeline,
    pub transparent_pipeline: vk::Pipeline,
    pub shadow_pipeline: vk::Pipeline,
}

impl Scene3dPipelineObjects {
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
                .chain(InstanceVertex::get_attribute_descriptions().into_iter())
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

        Ok(Self {
            opaque_pipeline: opaque_pipeline_builder.build(device).unwrap(),
            transparent_pipeline: transparent_pipeline_builder.build(device).unwrap(),
            shadow_pipeline: shadow_pipeline_builder.build(device).unwrap(),
        })
    }
}

struct Scene3dPass {
    pub resources: Scene3dResources,
    pub pipeline_objects: Scene3dPipelineObjects,
}

impl Scene3dPass {
    pub fn new(
        device: &ash::Device,
        allocator: &vk_mem::Allocator,
        command_pool: vk::CommandPool,
        queue: vk::Queue,
        queue_family_index: u32,
        descriptor_pool: vk::DescriptorPool,
        scene_descriptor_layout: vk::DescriptorSetLayout,
        window_extent: vk::Extent2D,
        pipeline_layout: vk::PipelineLayout,
        surface_format: vk::SurfaceFormatKHR,
    ) -> VkResult<Self> {
        let resources = Scene3dResources::new(
            device,
            allocator,
            command_pool,
            queue,
            queue_family_index,
            descriptor_pool,
            scene_descriptor_layout,
            window_extent,
        )?;

        let pipeline_objects =
            Scene3dPipelineObjects::new(device, pipeline_layout, surface_format.format)?;

        Ok(Self {
            resources,
            pipeline_objects,
        })
    }

    pub fn target_resized(
        &mut self,
        device: &ash::Device,
        allocator: &vk_mem::Allocator,
        command_pool: vk::CommandPool,
        queue: vk::Queue,
        window_extent: vk::Extent2D,
    ) -> VkResult<()> {
        self.resources
            .target_resized(device, allocator, command_pool, queue, window_extent)
    }
}
