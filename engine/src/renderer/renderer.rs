use std::borrow::Cow;
use std::{array, ffi, mem, ptr};

use ash::{khr, vk};
use glam::{Mat3, Mat4, Quat, Vec2, Vec3, Vec4, vec2, vec4};
use vk_mem::Alloc;
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle};
use winit::window::Window;

use crate::freetype::FreetypeLibrary;
use crate::renderer::images::{ImageTransition, transition_images};
use crate::renderer::mesh::{
    Global2DUniform, Global3DUniform, InstanceVertex, MaterialFlags, MaterialUniform, MeshVertex,
    SceneUniform, Vertex2d,
};
use crate::renderer::pipelines::RendererPipelineObjects;
use crate::renderer::text::GlyphAtlas;
use crate::scene::RenderScene;

const USE_VALIDATION_LAYERS: bool = true;
const MAX_FRAMES: usize = 1;
const SHADOW_MAP_RESOLUTION: u32 = 1024;
const MAX_INSTANCE_COUNT: usize = 10000;
const MAX_VERTICES_2D: usize = 50000;

const DESCRIPTOR_RATIOS: &[(vk::DescriptorType, u32)] = &[
    (vk::DescriptorType::COMBINED_IMAGE_SAMPLER, 1),
    (vk::DescriptorType::UNIFORM_BUFFER, 2),
];

unsafe extern "system" fn debug_messager_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _userdata: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    unsafe {
        if !message_severity.contains(vk::DebugUtilsMessageSeverityFlagsEXT::ERROR)
            && !message_severity.contains(vk::DebugUtilsMessageSeverityFlagsEXT::WARNING)
        {
            return vk::FALSE;
        }

        let callback_data = *callback_data;
        let message_id_number = callback_data.message_id_number;

        let message_id_name = if callback_data.p_message_id_name.is_null() {
            Cow::from("")
        } else {
            ffi::CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy()
        };

        let message = if callback_data.p_message.is_null() {
            Cow::from("")
        } else {
            ffi::CStr::from_ptr(callback_data.p_message).to_string_lossy()
        };

        let bt = std::backtrace::Backtrace::capture();

        println!(
            "{message_severity:?}:\n{message_type:?} [{message_id_name} ({message_id_number})]: {message}\n{bt}",
        );

        vk::FALSE
    }
}

struct GlobalDescriptors {
    global2d_buffer: (vk::Buffer, vk_mem::Allocation),
    camera_buffer: (vk::Buffer, vk_mem::Allocation),
    scene_buffer: (vk::Buffer, vk_mem::Allocation),
    main_pass_descriptor_set: vk::DescriptorSet,
    shadow_pass_descriptor_set: vk::DescriptorSet,
    shadow_map_sampler: vk::Sampler,
    skybox_dirty: bool,
    main_2d_pass_descriptor_set: vk::DescriptorSet,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct MeshHandle(u32);

impl GlobalDescriptors {
    fn new(
        device: &ash::Device,
        allocator: &vk_mem::Allocator,
        descriptor_pool: vk::DescriptorPool,
        frame_layout_3d: vk::DescriptorSetLayout,
        frame_layout_2d: vk::DescriptorSetLayout,
        shadow_map_view: vk::ImageView,
    ) -> Self {
        let layouts = [frame_layout_3d, frame_layout_3d, frame_layout_2d];
        let descriptor_set_alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&layouts);

        let descriptor_sets = unsafe {
            device
                .allocate_descriptor_sets(&descriptor_set_alloc_info)
                .unwrap()
        };

        let main_pass_descriptor_set = descriptor_sets[0];
        let shadow_pass_descriptor_set = descriptor_sets[1];
        let main_2d_pass_descriptor_set = descriptor_sets[2];

        let camera_uniform_buffer_info = vk::BufferCreateInfo::default()
            .usage(vk::BufferUsageFlags::UNIFORM_BUFFER)
            .size(mem::size_of::<Global3DUniform>() as u64);

        let global2d_uniform_buffer_info = vk::BufferCreateInfo::default()
            .usage(vk::BufferUsageFlags::UNIFORM_BUFFER)
            .size(mem::size_of::<Global2DUniform>() as u64);

        let scene_uniform_buffer_info = vk::BufferCreateInfo::default()
            .usage(vk::BufferUsageFlags::UNIFORM_BUFFER)
            .size(mem::size_of::<SceneUniform>() as u64);

        let uniform_alloc_info = vk_mem::AllocationCreateInfo {
            flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE
                | vk_mem::AllocationCreateFlags::MAPPED,
            usage: vk_mem::MemoryUsage::AutoPreferHost,

            ..Default::default()
        };

        let camera_buffer = unsafe {
            allocator
                .create_buffer(&camera_uniform_buffer_info, &uniform_alloc_info)
                .unwrap()
        };

        let scene_buffer = unsafe {
            allocator
                .create_buffer(&scene_uniform_buffer_info, &uniform_alloc_info)
                .unwrap()
        };

        let global2d_buffer = unsafe {
            allocator
                .create_buffer(&global2d_uniform_buffer_info, &uniform_alloc_info)
                .unwrap()
        };

        let camera_buffer_info = [vk::DescriptorBufferInfo::default()
            .offset(0)
            .range(mem::size_of::<Global3DUniform>() as u64)
            .buffer(camera_buffer.0)];

        let scene_buffer_info = [vk::DescriptorBufferInfo::default()
            .offset(0)
            .range(mem::size_of::<SceneUniform>() as u64)
            .buffer(scene_buffer.0)];

        let global2d_buffer_info = [vk::DescriptorBufferInfo::default()
            .offset(0)
            .range(mem::size_of::<Global2DUniform>() as u64)
            .buffer(global2d_buffer.0)];

        let shadow_map_sampler_info = vk::SamplerCreateInfo::default()
            .mag_filter(vk::Filter::LINEAR)
            .min_filter(vk::Filter::LINEAR)
            .compare_enable(true)
            .compare_op(vk::CompareOp::GREATER)
            .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_BORDER)
            .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_BORDER)
            .border_color(vk::BorderColor::FLOAT_OPAQUE_BLACK);

        let shadow_map_sampler = unsafe {
            device
                .create_sampler(&shadow_map_sampler_info, None)
                .unwrap()
        };

        let shadow_map_image_info = [vk::DescriptorImageInfo::default()
            .image_view(shadow_map_view)
            .image_layout(vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL)
            .sampler(shadow_map_sampler)];

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
            vk::WriteDescriptorSet::default()
                .dst_set(main_2d_pass_descriptor_set)
                .dst_binding(0)
                .dst_array_element(0)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(&global2d_buffer_info),
        ];

        unsafe { device.update_descriptor_sets(&descriptor_write, &[]) };

        GlobalDescriptors {
            global2d_buffer,
            camera_buffer,
            scene_buffer,
            main_pass_descriptor_set,
            shadow_pass_descriptor_set,
            main_2d_pass_descriptor_set,
            shadow_map_sampler,
            skybox_dirty: true,
        }
    }

    fn update_skybox(&mut self, device: &ash::Device, skybox_texture: &Texture) {
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

    fn destroy(&mut self, device: &ash::Device, allocator: &vk_mem::Allocator) {
        unsafe { allocator.destroy_buffer(self.camera_buffer.0, &mut self.camera_buffer.1) };
        unsafe { allocator.destroy_buffer(self.scene_buffer.0, &mut self.scene_buffer.1) };
        unsafe { allocator.destroy_buffer(self.global2d_buffer.0, &mut self.global2d_buffer.1) };
        unsafe { device.destroy_sampler(self.shadow_map_sampler, None) };
    }
}

struct RenderFrame {
    command_pool: vk::CommandPool,
    command_buffer: vk::CommandBuffer,
    swapchain_semaphore: vk::Semaphore,
    in_flight_fence: vk::Fence,
    per_frame_descriptor_data: GlobalDescriptors,
    depth_image_view: vk::ImageView,
    depth_image: (vk::Image, vk_mem::Allocation),
    instance_buffer: (vk::Buffer, vk_mem::Allocation),
    vertex2d_buffer: (vk::Buffer, vk_mem::Allocation),
    vertex2d_index_buffer: (vk::Buffer, vk_mem::Allocation),
    shadow_map: (vk::Image, vk_mem::Allocation),
    shadow_map_view: vk::ImageView,
}

impl RenderFrame {
    fn new(
        device: &ash::Device,
        allocator: &vk_mem::Allocator,
        queue: vk::Queue,
        descriptor_pool: vk::DescriptorPool,
        descriptor_layouts: &DescriptorSetLayouts,
        window_extent: vk::Extent2D,
        queue_family_index: u32,
    ) -> Self {
        let command_pool = create_command_pool(device, queue_family_index);

        let command_buffer_alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        let command_buffer = unsafe {
            device
                .allocate_command_buffers(&command_buffer_alloc_info)
                .unwrap()[0]
        };

        let semaphore_create_info =
            vk::SemaphoreCreateInfo::default().flags(vk::SemaphoreCreateFlags::empty());

        let fence_create_info =
            vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);

        let swapchain_semaphore = unsafe {
            device
                .create_semaphore(&semaphore_create_info, None)
                .unwrap()
        };

        let in_flight_fence = unsafe { device.create_fence(&fence_create_info, None).unwrap() };

        let (depth_image, depth_image_view) = Self::create_depth_image(
            device,
            allocator,
            queue,
            command_pool,
            window_extent,
            vk::ImageUsageFlags::empty(),
        );

        let (shadow_map, shadow_map_view) = Self::create_depth_image(
            device,
            allocator,
            queue,
            command_pool,
            vk::Extent2D {
                width: SHADOW_MAP_RESOLUTION,
                height: SHADOW_MAP_RESOLUTION,
            },
            vk::ImageUsageFlags::SAMPLED,
        );

        let per_frame_descriptor_data = GlobalDescriptors::new(
            device,
            allocator,
            descriptor_pool,
            descriptor_layouts.global_3d_layout,
            descriptor_layouts.global_2d_layout,
            shadow_map_view,
        );

        let instance_buffer = create_instance_buffer(allocator);
        let (vertex2d_buffer, vertex2d_index_buffer) = create_vertex2d_buffer(allocator);

        RenderFrame {
            command_pool,
            command_buffer,
            swapchain_semaphore,
            in_flight_fence,
            per_frame_descriptor_data,
            depth_image,
            depth_image_view,
            shadow_map,
            shadow_map_view,
            instance_buffer,
            vertex2d_buffer,
            vertex2d_index_buffer,
        }
    }

    fn handle_resize(
        &mut self,
        device: &ash::Device,
        allocator: &vk_mem::Allocator,
        command_pool: vk::CommandPool,
        queue: vk::Queue,
        window_extent: vk::Extent2D,
    ) {
        unsafe {
            device.destroy_semaphore(self.swapchain_semaphore, None);
        };

        let semaphore_create_info =
            vk::SemaphoreCreateInfo::default().flags(vk::SemaphoreCreateFlags::empty());

        let new_semaphore = unsafe {
            device
                .create_semaphore(&semaphore_create_info, None)
                .unwrap()
        };

        unsafe {
            allocator.destroy_image(self.depth_image.0, &mut self.depth_image.1);
            device.destroy_image_view(self.depth_image_view, None);
        };

        let (depth_image, depth_image_view) = Self::create_depth_image(
            device,
            allocator,
            queue,
            command_pool,
            window_extent,
            vk::ImageUsageFlags::empty(),
        );

        self.swapchain_semaphore = new_semaphore;
        self.depth_image = depth_image;
        self.depth_image_view = depth_image_view;
    }

    fn create_depth_image(
        device: &ash::Device,
        allocator: &vk_mem::Allocator,
        queue: vk::Queue,
        command_pool: vk::CommandPool,
        extent: vk::Extent2D,
        extra_usage_flags: vk::ImageUsageFlags,
    ) -> ((vk::Image, vk_mem::Allocation), vk::ImageView) {
        let image_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .extent(vk::Extent3D {
                width: extent.width,
                height: extent.height,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .format(vk::Format::D32_SFLOAT)
            .tiling(vk::ImageTiling::OPTIMAL)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT | extra_usage_flags)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .samples(vk::SampleCountFlags::TYPE_1);

        let image_alloc_info = vk_mem::AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::AutoPreferDevice,
            preferred_flags: vk::MemoryPropertyFlags::LAZILY_ALLOCATED,
            ..Default::default()
        };

        let depth_image = unsafe {
            allocator
                .create_image(&image_info, &image_alloc_info)
                .unwrap()
        };

        let command_buffers = unsafe {
            device
                .allocate_command_buffers(
                    &vk::CommandBufferAllocateInfo::default()
                        .command_pool(command_pool)
                        .command_buffer_count(1)
                        .level(vk::CommandBufferLevel::PRIMARY),
                )
                .unwrap()
        };

        unsafe {
            device
                .begin_command_buffer(
                    command_buffers[0],
                    &vk::CommandBufferBeginInfo::default()
                        .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
                )
                .unwrap()
        };

        transition_image(
            device,
            command_buffers[0],
            depth_image.0,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
            vk::ImageAspectFlags::DEPTH,
        );

        unsafe {
            device.end_command_buffer(command_buffers[0]).unwrap();

            device
                .queue_submit(
                    queue,
                    &[vk::SubmitInfo::default().command_buffers(&command_buffers)],
                    vk::Fence::null(),
                )
                .unwrap();

            device.queue_wait_idle(queue).unwrap();

            device.free_command_buffers(command_pool, &command_buffers);
        }

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

        let depth_image_view = unsafe { device.create_image_view(&image_view_info, None).unwrap() };

        (depth_image, depth_image_view)
    }

    fn destroy(&mut self, device: &ash::Device, allocator: &vk_mem::Allocator) {
        self.per_frame_descriptor_data.destroy(device, allocator);
        unsafe { allocator.destroy_image(self.depth_image.0, &mut self.depth_image.1) };
        unsafe { allocator.destroy_image(self.shadow_map.0, &mut self.shadow_map.1) };
        unsafe { allocator.destroy_buffer(self.instance_buffer.0, &mut self.instance_buffer.1) };
        unsafe { allocator.destroy_buffer(self.vertex2d_buffer.0, &mut self.vertex2d_buffer.1) };
        unsafe {
            allocator.destroy_buffer(
                self.vertex2d_index_buffer.0,
                &mut self.vertex2d_index_buffer.1,
            )
        };
    }
}

struct DescriptorSetLayouts {
    global_3d_layout: vk::DescriptorSetLayout,
    texture_layout: vk::DescriptorSetLayout,
    material_layout: vk::DescriptorSetLayout,
    global_2d_layout: vk::DescriptorSetLayout,
}

struct TextureDescriptors {
    texture: Texture,
    descriptor_set: vk::DescriptorSet,
}

impl TextureDescriptors {
    fn create_texture(
        device: &ash::Device,
        descriptor_set_layouts: &DescriptorSetLayouts,
        descriptor_pool: vk::DescriptorPool,
        texture: Texture,
        sampler: Option<vk::Sampler>, // allow providing a sampler which the texture does not own
    ) -> Self {
        let layouts = &[descriptor_set_layouts.texture_layout];

        let descriptor_alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(layouts);

        let descriptor_sets = unsafe {
            device
                .allocate_descriptor_sets(&descriptor_alloc_info)
                .unwrap()
        };

        let descriptor_set = descriptor_sets[0];

        let descriptor_image_info = &[vk::DescriptorImageInfo::default()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image_view(texture.image_view)
            .sampler(sampler.or(texture.sampler).expect("no sampler provided"))];

        let descriptor_writes = &[vk::WriteDescriptorSet::default()
            .image_info(descriptor_image_info)
            .descriptor_count(1)
            .dst_binding(0)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .dst_set(descriptor_set)];

        unsafe { device.update_descriptor_sets(descriptor_writes, &[]) };

        Self {
            texture,
            descriptor_set,
        }
    }

    fn destroy(&mut self, device: &ash::Device, allocator: &vk_mem::Allocator) {
        self.texture.destroy(device, allocator);
    }
}

struct MeshBuffer {
    vertex_buffer: (vk::Buffer, vk_mem::Allocation),
    index_buffer: (vk::Buffer, vk_mem::Allocation),
    index_count: u32,
}

impl MeshBuffer {
    fn allocate_mesh(
        device: &ash::Device,
        allocator: &vk_mem::Allocator,
        queue: vk::Queue,
        command_pool: vk::CommandPool,
        vertices: &[MeshVertex],
        indicies: &[u16],
    ) -> Self {
        let vb_size = mem::size_of_val(vertices) as u64;
        let ib_size = mem::size_of_val(indicies) as u64;

        let alloc_info = vk_mem::AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::AutoPreferHost,
            ..Default::default()
        };

        let vertex_buffer = unsafe {
            allocator
                .create_buffer(
                    &vk::BufferCreateInfo::default().size(vb_size).usage(
                        vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
                    ),
                    &alloc_info,
                )
                .unwrap()
        };

        let index_buffer = unsafe {
            allocator
                .create_buffer(
                    &vk::BufferCreateInfo::default().size(ib_size).usage(
                        vk::BufferUsageFlags::INDEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
                    ),
                    &alloc_info,
                )
                .unwrap()
        };

        let staging_alloc_info = vk_mem::AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::Auto,
            flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE
                | vk_mem::AllocationCreateFlags::MAPPED,
            ..Default::default()
        };

        let mut vertex_staging_buffer = unsafe {
            allocator
                .create_buffer(
                    &vk::BufferCreateInfo::default()
                        .usage(vk::BufferUsageFlags::TRANSFER_SRC)
                        .size(vb_size),
                    &staging_alloc_info,
                )
                .unwrap()
        };

        let mut index_staging_buffer = unsafe {
            allocator
                .create_buffer(
                    &vk::BufferCreateInfo::default()
                        .usage(vk::BufferUsageFlags::TRANSFER_SRC)
                        .size(ib_size),
                    &staging_alloc_info,
                )
                .unwrap()
        };

        let vertex_alloc_info = allocator.get_allocation_info(&vertex_staging_buffer.1);
        let index_alloc_info = allocator.get_allocation_info(&index_staging_buffer.1);

        unsafe {
            std::ptr::copy_nonoverlapping(
                vertices.as_ptr(),
                vertex_alloc_info.mapped_data.cast(),
                vertices.len(),
            );

            std::ptr::copy_nonoverlapping(
                indicies.as_ptr(),
                index_alloc_info.mapped_data.cast(),
                indicies.len(),
            );
        }

        let command_buffer_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        let command_buffers = unsafe {
            device
                .allocate_command_buffers(&command_buffer_info)
                .unwrap()
        };

        let command_buffer = command_buffers[0];

        let being_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe {
            device
                .begin_command_buffer(command_buffer, &being_info)
                .unwrap();

            device.cmd_copy_buffer(
                command_buffer,
                vertex_staging_buffer.0,
                vertex_buffer.0,
                &[vk::BufferCopy::default()
                    .src_offset(0)
                    .dst_offset(0)
                    .size(vb_size)],
            );

            device.cmd_copy_buffer(
                command_buffer,
                index_staging_buffer.0,
                index_buffer.0,
                &[vk::BufferCopy::default()
                    .src_offset(0)
                    .dst_offset(0)
                    .size(ib_size)],
            );

            device.end_command_buffer(command_buffer).unwrap();

            device
                .queue_submit(
                    queue,
                    &[vk::SubmitInfo::default().command_buffers(&command_buffers)],
                    vk::Fence::null(),
                )
                .unwrap();

            device.queue_wait_idle(queue).unwrap();

            allocator.destroy_buffer(index_staging_buffer.0, &mut index_staging_buffer.1);
            allocator.destroy_buffer(vertex_staging_buffer.0, &mut vertex_staging_buffer.1);
        };

        Self {
            vertex_buffer,
            index_buffer,
            index_count: indicies.len() as u32,
        }
    }

    fn destroy(&mut self, allocator: &vk_mem::Allocator) {
        unsafe {
            allocator.destroy_buffer(self.vertex_buffer.0, &mut self.vertex_buffer.1);
            allocator.destroy_buffer(self.index_buffer.0, &mut self.index_buffer.1);
        }
    }
}

struct Texture {
    image: (vk::Image, vk_mem::Allocation),
    image_view: vk::ImageView,
    sampler: Option<vk::Sampler>,
}

impl Texture {
    fn from_rgba_data(
        device: &ash::Device,
        allocator: &vk_mem::Allocator,
        queue: vk::Queue,
        command_pool: vk::CommandPool,
        width: u32,
        height: u32,
        data: &[u8],
    ) -> Self {
        let image_extent = vk::Extent3D {
            width,
            height,
            depth: 1,
        };

        let format = vk::Format::R8G8B8A8_SRGB;

        let image_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .extent(image_extent)
            .mip_levels(1)
            .array_layers(1)
            .format(format)
            .tiling(vk::ImageTiling::OPTIMAL)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .usage(
                vk::ImageUsageFlags::SAMPLED
                    | vk::ImageUsageFlags::TRANSFER_DST
                    | vk::ImageUsageFlags::TRANSFER_SRC,
            )
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .samples(vk::SampleCountFlags::TYPE_1)
            .flags(vk::ImageCreateFlags::empty());

        let image_alloc_create_info = vk_mem::AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::AutoPreferDevice,
            ..Default::default()
        };

        let image = unsafe {
            allocator
                .create_image(&image_info, &image_alloc_create_info)
                .unwrap()
        };

        let image_view_info = vk::ImageViewCreateInfo::default()
            .image(image.0)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            })
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(format);

        let image_view = unsafe { device.create_image_view(&image_view_info, None).unwrap() };

        let buffer_info = vk::BufferCreateInfo::default()
            .usage(vk::BufferUsageFlags::TRANSFER_SRC)
            .size(width as u64 * height as u64 * 4);

        let create_info = vk_mem::AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::Auto,
            flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE
                | vk_mem::AllocationCreateFlags::MAPPED,
            ..Default::default()
        };

        let mut staging_buffer =
            unsafe { allocator.create_buffer(&buffer_info, &create_info).unwrap() };
        let alloc_info = allocator.get_allocation_info(&staging_buffer.1);

        unsafe {
            ptr::copy_nonoverlapping(data.as_ptr(), alloc_info.mapped_data.cast(), data.len())
        };

        let command_buffer_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        let command_buffers = unsafe {
            device
                .allocate_command_buffers(&command_buffer_info)
                .unwrap()
        };

        let command_buffer = command_buffers[0];

        let being_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe {
            device
                .begin_command_buffer(command_buffer, &being_info)
                .unwrap()
        };

        transition_image(
            device,
            command_buffer,
            image.0,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageAspectFlags::COLOR,
        );

        let copy_regions = &[vk::BufferImageCopy {
            buffer_offset: 0,
            buffer_row_length: 0,
            buffer_image_height: 0,
            image_subresource: vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            },
            image_offset: vk::Offset3D::default(),
            image_extent,
        }];

        unsafe {
            device.cmd_copy_buffer_to_image(
                command_buffer,
                staging_buffer.0,
                image.0,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                copy_regions,
            )
        };

        transition_image(
            device,
            command_buffer,
            image.0,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            vk::ImageAspectFlags::COLOR,
        );

        unsafe { device.end_command_buffer(command_buffer).unwrap() };

        unsafe {
            device
                .queue_submit(
                    queue,
                    &[vk::SubmitInfo::default().command_buffers(&command_buffers)],
                    vk::Fence::null(),
                )
                .unwrap();

            device.queue_wait_idle(queue).unwrap();

            allocator.destroy_buffer(staging_buffer.0, &mut staging_buffer.1);
        }

        Self {
            image,
            image_view,
            sampler: None,
        }
    }

    fn from_skybox_data(
        device: &ash::Device,
        allocator: &vk_mem::Allocator,
        queue: vk::Queue,
        command_pool: vk::CommandPool,
        sampler_filter: vk::Filter,
        skybox_data: &SkyboxImageData,
    ) -> Self {
        let mut all_image_data = Vec::new();
        all_image_data.extend_from_slice(skybox_data.right);
        all_image_data.extend_from_slice(skybox_data.left);
        all_image_data.extend_from_slice(skybox_data.top);
        all_image_data.extend_from_slice(skybox_data.bottom);
        all_image_data.extend_from_slice(skybox_data.back);
        all_image_data.extend_from_slice(skybox_data.front);

        let image_extent = vk::Extent3D {
            width: skybox_data.width,
            height: skybox_data.height,
            depth: 1,
        };

        let format = vk::Format::R8G8B8A8_SRGB;

        let image_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .extent(image_extent)
            .mip_levels(1)
            .array_layers(6)
            .format(format)
            .tiling(vk::ImageTiling::OPTIMAL)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .usage(
                vk::ImageUsageFlags::SAMPLED
                    | vk::ImageUsageFlags::TRANSFER_DST
                    | vk::ImageUsageFlags::TRANSFER_SRC,
            )
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .samples(vk::SampleCountFlags::TYPE_1)
            .flags(vk::ImageCreateFlags::CUBE_COMPATIBLE);

        let image_alloc_create_info = vk_mem::AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::AutoPreferDevice,
            ..Default::default()
        };

        let image = unsafe {
            allocator
                .create_image(&image_info, &image_alloc_create_info)
                .unwrap()
        };

        let image_view_info = vk::ImageViewCreateInfo::default()
            .image(image.0)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 6,
            })
            .view_type(vk::ImageViewType::CUBE)
            .format(format);

        let image_view = unsafe { device.create_image_view(&image_view_info, None).unwrap() };

        let buffer_info = vk::BufferCreateInfo::default()
            .usage(vk::BufferUsageFlags::TRANSFER_SRC)
            .size(all_image_data.len() as u64);

        let create_info = vk_mem::AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::Auto,
            flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE
                | vk_mem::AllocationCreateFlags::MAPPED,
            ..Default::default()
        };

        let mut staging_buffer =
            unsafe { allocator.create_buffer(&buffer_info, &create_info).unwrap() };
        let alloc_info = allocator.get_allocation_info(&staging_buffer.1);

        unsafe {
            ptr::copy_nonoverlapping(
                all_image_data.as_ptr(),
                alloc_info.mapped_data.cast(),
                all_image_data.len(),
            );
        }

        let command_buffer_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        let command_buffers = unsafe {
            device
                .allocate_command_buffers(&command_buffer_info)
                .unwrap()
        };

        let command_buffer = command_buffers[0];

        let being_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe {
            device
                .begin_command_buffer(command_buffer, &being_info)
                .unwrap()
        };

        transition_image(
            device,
            command_buffer,
            image.0,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageAspectFlags::COLOR,
        );

        let img_stride = skybox_data.width as u64 * skybox_data.height as u64 * 4;

        let copy_regions: [vk::BufferImageCopy; 6] = array::from_fn(|i| vk::BufferImageCopy {
            buffer_offset: img_stride * i as u64,
            buffer_row_length: 0,
            buffer_image_height: 0,
            image_subresource: vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: i as u32,
                layer_count: 1,
            },
            image_offset: vk::Offset3D { x: 0, y: 0, z: 0 },
            image_extent,
        });

        unsafe {
            device.cmd_copy_buffer_to_image(
                command_buffer,
                staging_buffer.0,
                image.0,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &copy_regions,
            )
        };

        transition_image(
            device,
            command_buffer,
            image.0,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            vk::ImageAspectFlags::COLOR,
        );

        unsafe { device.end_command_buffer(command_buffer).unwrap() };

        unsafe {
            device
                .queue_submit(
                    queue,
                    &[vk::SubmitInfo::default().command_buffers(&command_buffers)],
                    vk::Fence::null(),
                )
                .unwrap();

            device.queue_wait_idle(queue).unwrap();

            allocator.destroy_buffer(staging_buffer.0, &mut staging_buffer.1);
        }

        let sampler_info = vk::SamplerCreateInfo::default()
            .mag_filter(sampler_filter)
            .min_filter(sampler_filter)
            .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
            .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .min_lod(0.0)
            .max_lod(vk::LOD_CLAMP_NONE)
            .mip_lod_bias(0.0)
            .anisotropy_enable(false)
            .compare_enable(false)
            .unnormalized_coordinates(false);

        let sampler = unsafe { device.create_sampler(&sampler_info, None).unwrap() };

        Texture {
            image,
            image_view,
            sampler: Some(sampler),
        }
    }

    fn destroy(&mut self, device: &ash::Device, allocator: &vk_mem::Allocator) {
        unsafe {
            device.destroy_image_view(self.image_view, None);

            if let Some(sampler) = self.sampler {
                device.destroy_sampler(sampler, None);
            }

            allocator.destroy_image(self.image.0, &mut self.image.1);
        }
    }
}

#[derive(Debug)]
struct MeshBatch {
    mesh_id: MeshHandle,
    material_id: u32,
    instance_offset: u64,
    instance_count: u32,
    is_opaque: bool,
}

struct MaterialDescriptor {
    uniform_buffer: (vk::Buffer, vk_mem::Allocation),
    descriptor_set: vk::DescriptorSet,
}

impl MaterialDescriptor {
    fn new(
        device: &ash::Device,
        allocator: &vk_mem::Allocator,
        queue: vk::Queue,
        command_pool: vk::CommandPool,
        descriptor_pool: vk::DescriptorPool,
        descriptor_set_layouts: &DescriptorSetLayouts,
        uv_scale: Vec2,
        shininess: f32,
    ) -> Self {
        let layout = &[descriptor_set_layouts.material_layout];

        let descriptor_info = vk::DescriptorSetAllocateInfo::default()
            .set_layouts(layout)
            .descriptor_pool(descriptor_pool);

        let descriptor_sets = unsafe { device.allocate_descriptor_sets(&descriptor_info).unwrap() };
        let descriptor_set = descriptor_sets[0];

        let uniform_buffer_info = vk::BufferCreateInfo::default()
            .usage(vk::BufferUsageFlags::UNIFORM_BUFFER)
            .size(mem::size_of::<MaterialUniform>() as u64);

        let alloc_create_info = vk_mem::AllocationCreateInfo {
            flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE
                | vk_mem::AllocationCreateFlags::MAPPED,
            usage: vk_mem::MemoryUsage::AutoPreferHost,
            ..Default::default()
        };

        let uniform_buffer = unsafe {
            allocator
                .create_buffer(&uniform_buffer_info, &alloc_create_info)
                .unwrap()
        };

        let alloc_info = allocator.get_allocation_info(&uniform_buffer.1);

        let material_uniform = MaterialUniform {
            uv_scale,
            shininess,
            flags: MaterialFlags::ModelSpace,
        };

        unsafe {
            std::ptr::copy_nonoverlapping(&material_uniform, alloc_info.mapped_data.cast(), 1);
        };

        let descriptor_buffer_info = [vk::DescriptorBufferInfo::default()
            .buffer(uniform_buffer.0)
            .offset(0)
            .range(mem::size_of::<MaterialUniform>() as u64)];

        let descriptor_writes = [vk::WriteDescriptorSet::default()
            .dst_set(descriptor_set)
            .dst_array_element(0)
            .dst_binding(0)
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .buffer_info(&descriptor_buffer_info)];

        unsafe { device.update_descriptor_sets(&descriptor_writes, &[]) };

        Self {
            uniform_buffer,
            descriptor_set,
        }
    }

    fn destroy(&mut self, allocator: &vk_mem::Allocator) {
        unsafe { allocator.destroy_buffer(self.uniform_buffer.0, &mut self.uniform_buffer.1) };
    }
}

pub struct VulkanContext {
    entry: ash::Entry,
    instance: ash::Instance,
    surface_loader: khr::surface::Instance,
    physical_device: vk::PhysicalDevice,
    device: ash::Device,
    swapchain_loader: khr::swapchain::Device,
    allocator: vk_mem::Allocator,
    graphics_queue: vk::Queue,
    graphics_queue_family_index: u32,
    command_pool: vk::CommandPool,

    current_frame: usize,
    should_resize: bool,

    surface: vk::SurfaceKHR,
    surface_format: vk::SurfaceFormatKHR,
    swapchain_extent: vk::Extent2D,
    swapchain: vk::SwapchainKHR,
    swapchain_images: Vec<vk::Image>,
    swapchain_image_views: Vec<vk::ImageView>,
    submit_semaphores: Vec<vk::Semaphore>,
    render_frames: Vec<RenderFrame>,

    descriptor_set_layouts: DescriptorSetLayouts,
    material_descriptors: Vec<MaterialDescriptor>,
    descriptor_pool: vk::DescriptorPool,
    pipeline_layout_3d: vk::PipelineLayout,
    pipeline_objects: RendererPipelineObjects,
    pipeline_layout_2d: vk::PipelineLayout,

    mesh_buffers: Vec<MeshBuffer>,
    textures: Vec<TextureDescriptors>,
    skybox_textures: Vec<Texture>,
    current_skybox: Option<u32>,

    glyph_atlas: GlyphAtlas,
}

pub struct SkyboxImageData<'a> {
    pub width: u32,
    pub height: u32,
    pub top: &'a [u8],
    pub bottom: &'a [u8],
    pub front: &'a [u8],
    pub back: &'a [u8],
    pub left: &'a [u8],
    pub right: &'a [u8],
}

fn create_instance(entry: &ash::Entry, raw_display_handle: RawDisplayHandle) -> ash::Instance {
    let mut extensions = vec![ash::ext::debug_utils::NAME.as_ptr()];
    let mut validation_layers = vec![];

    if USE_VALIDATION_LAYERS {
        validation_layers.push(c"VK_LAYER_KHRONOS_validation".as_ptr())
    }

    let surface_extensions = ash_window::enumerate_required_extensions(raw_display_handle).unwrap();

    extensions.extend_from_slice(surface_extensions);

    let appinfo = vk::ApplicationInfo::default()
        .application_name(c"HexxEngine")
        .api_version(ash::vk::API_VERSION_1_3);

    let create_info = vk::InstanceCreateInfo::default()
        .application_info(&appinfo)
        .enabled_extension_names(&extensions)
        .enabled_layer_names(&validation_layers);

    let instance = unsafe { entry.create_instance(&create_info, None).unwrap() };
    let debug_utils_fn = ash::ext::debug_utils::Instance::new(entry, &instance);

    let messager_create_info = vk::DebugUtilsMessengerCreateInfoEXT::default()
        .message_severity(
            vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                | vk::DebugUtilsMessageSeverityFlagsEXT::INFO,
        )
        .message_type(
            vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
                | vk::DebugUtilsMessageTypeFlagsEXT::DEVICE_ADDRESS_BINDING,
        )
        .pfn_user_callback(Some(debug_messager_callback));

    unsafe {
        debug_utils_fn
            .create_debug_utils_messenger(&messager_create_info, None)
            .unwrap()
    };

    instance
}

fn create_swapchain(
    device: &ash::Device,
    surface_fn: &khr::surface::Instance,
    swapchain_fn: &khr::swapchain::Device,
    physical_device: vk::PhysicalDevice,
    surface: vk::SurfaceKHR,
    surface_format: vk::SurfaceFormatKHR,
    window_size: (u32, u32),
    old_swapchain: Option<vk::SwapchainKHR>,
) -> Result<
    (
        vk::SwapchainKHR,
        Vec<vk::Image>,
        Vec<vk::ImageView>,
        vk::Extent2D,
    ),
    vk::Result,
> {
    let surface_capabilities = unsafe {
        surface_fn
            .get_physical_device_surface_capabilities(physical_device, surface)
            .unwrap()
    };

    let surface_max_image_extent = surface_capabilities.max_image_extent;

    let image_extent = if surface_max_image_extent.width != u32::MAX {
        surface_max_image_extent
    } else {
        vk::Extent2D {
            width: window_size.0,
            height: window_size.1,
        }
    };

    let create_swapchain_info = vk::SwapchainCreateInfoKHR::default()
        .surface(surface)
        .image_format(surface_format.format)
        .image_color_space(surface_format.color_space)
        .present_mode(vk::PresentModeKHR::FIFO)
        .image_array_layers(1)
        .min_image_count(surface_capabilities.min_image_count + 1)
        .pre_transform(surface_capabilities.current_transform)
        .clipped(true)
        .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
        .image_usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::COLOR_ATTACHMENT)
        .image_extent(image_extent)
        .old_swapchain(old_swapchain.unwrap_or(vk::SwapchainKHR::null()));

    let swapchain = unsafe { swapchain_fn.create_swapchain(&create_swapchain_info, None)? };

    let swapchain_images = unsafe { swapchain_fn.get_swapchain_images(swapchain).unwrap() };

    let image_views: Vec<vk::ImageView> = swapchain_images
        .iter()
        .map(|image| {
            let image_create_info = vk::ImageViewCreateInfo::default()
                .image(*image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(surface_format.format)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: vk::REMAINING_MIP_LEVELS,
                    base_array_layer: 0,
                    layer_count: vk::REMAINING_ARRAY_LAYERS,
                });

            unsafe { device.create_image_view(&image_create_info, None).unwrap() }
        })
        .collect();

    Ok((swapchain, swapchain_images, image_views, image_extent))
}

fn transition_image(
    device: &ash::Device,
    command_buffer: vk::CommandBuffer,
    image: vk::Image,
    current_layout: vk::ImageLayout,
    new_layout: vk::ImageLayout,
    aspect_mask: vk::ImageAspectFlags,
) {
    let image_barriers = &[vk::ImageMemoryBarrier2::default()
        .src_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
        .src_access_mask(vk::AccessFlags2::MEMORY_WRITE)
        .dst_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
        .dst_access_mask(vk::AccessFlags2::MEMORY_WRITE | vk::AccessFlags2::MEMORY_READ)
        .old_layout(current_layout)
        .new_layout(new_layout)
        .subresource_range(vk::ImageSubresourceRange {
            aspect_mask,
            base_mip_level: 0,
            level_count: vk::REMAINING_MIP_LEVELS,
            base_array_layer: 0,
            layer_count: vk::REMAINING_ARRAY_LAYERS,
        })
        .image(image)];

    let dep_info = vk::DependencyInfo::default().image_memory_barriers(image_barriers);

    unsafe { device.cmd_pipeline_barrier2(command_buffer, &dep_info) };
}

fn create_nearest_sampler(device: &ash::Device) -> vk::Sampler {
    let sampler_info = vk::SamplerCreateInfo::default()
        .mag_filter(vk::Filter::NEAREST)
        .min_filter(vk::Filter::NEAREST);

    unsafe { device.create_sampler(&sampler_info, None).unwrap() }
}

fn create_command_pool(device: &ash::Device, queue_family_index: u32) -> vk::CommandPool {
    let command_pool_create_info = vk::CommandPoolCreateInfo::default()
        .queue_family_index(queue_family_index)
        .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);

    let command_pool = unsafe {
        device
            .create_command_pool(&command_pool_create_info, None)
            .unwrap()
    };

    command_pool
}

fn create_submit_semaphores(device: &ash::Device, count: usize) -> Vec<vk::Semaphore> {
    let semaphores: Vec<vk::Semaphore> = (0..count)
        .map(|_i| {
            let semaphore_create_info =
                vk::SemaphoreCreateInfo::default().flags(vk::SemaphoreCreateFlags::empty());

            let swapchain_semaphore = unsafe {
                device
                    .create_semaphore(&semaphore_create_info, None)
                    .unwrap()
            };

            swapchain_semaphore
        })
        .collect();

    semaphores
}

fn create_instance_buffer(allocator: &vk_mem::Allocator) -> (vk::Buffer, vk_mem::Allocation) {
    let instance_buffer_info = vk::BufferCreateInfo::default()
        .size((mem::size_of::<InstanceVertex>() * MAX_INSTANCE_COUNT) as u64)
        .usage(vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST);

    let alloc_info = vk_mem::AllocationCreateInfo {
        usage: vk_mem::MemoryUsage::AutoPreferHost,
        flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE
            | vk_mem::AllocationCreateFlags::MAPPED,
        ..Default::default()
    };

    let instance_buffer = unsafe {
        allocator
            .create_buffer(&instance_buffer_info, &alloc_info)
            .unwrap()
    };

    instance_buffer
}

fn create_vertex2d_buffer(
    allocator: &vk_mem::Allocator,
) -> (
    (vk::Buffer, vk_mem::Allocation),
    (vk::Buffer, vk_mem::Allocation),
) {
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

    let vertex_buffer = unsafe {
        allocator
            .create_buffer(&vertex_buffer_info, &alloc_info)
            .unwrap()
    };

    let index_buffer = unsafe {
        allocator
            .create_buffer(&index_buffer_info, &alloc_info)
            .unwrap()
    };

    (vertex_buffer, index_buffer)
}

impl VulkanContext {
    pub fn new(window: &Window) -> Self {
        let raw_window_handle = window.window_handle().unwrap().as_raw();
        let raw_display_handle = window.display_handle().unwrap().as_raw();

        let freetype = FreetypeLibrary::new().unwrap();

        let entry = unsafe { ash::Entry::load().unwrap() };
        let instance = create_instance(&entry, raw_display_handle);
        let surface_loader = ash::khr::surface::Instance::new(&entry, &instance);

        let surface = unsafe {
            ash_window::create_surface(
                &entry,
                &instance,
                raw_display_handle,
                raw_window_handle,
                None,
            )
            .unwrap()
        };

        let physical_devices = unsafe { instance.enumerate_physical_devices().unwrap() };

        // TODO: improve selection method
        let physical_device = physical_devices[0];
        let queue_family_properties =
            unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

        let graphics_queue_family_index = queue_family_properties
            .iter()
            .enumerate()
            .find_map(|(i, &props)| {
                let surface_support = unsafe {
                    surface_loader
                        .get_physical_device_surface_support(physical_device, i as u32, surface)
                        .unwrap_or(false)
                };

                if props.queue_flags.contains(vk::QueueFlags::GRAPHICS) && surface_support {
                    Some(i as u32)
                } else {
                    None
                }
            })
            .unwrap();

        let device_queue_create_infos = &[vk::DeviceQueueCreateInfo::default()
            .queue_family_index(graphics_queue_family_index)
            .queue_priorities(&[0.5])];

        let device_extensions = &[
            vk::KHR_SWAPCHAIN_NAME.as_ptr(),
            vk::KHR_SYNCHRONIZATION2_NAME.as_ptr(),
            vk::KHR_CREATE_RENDERPASS2_NAME.as_ptr(),
            vk::KHR_DYNAMIC_RENDERING_NAME.as_ptr(),
        ];

        let mut vk13_features = vk::PhysicalDeviceVulkan13Features::default()
            .dynamic_rendering(true)
            .synchronization2(true);

        let mut vk12_features =
            vk::PhysicalDeviceVulkan12Features::default().descriptor_indexing(true);

        let mut physical_device_features = vk::PhysicalDeviceFeatures2::default()
            .push_next(&mut vk13_features)
            .push_next(&mut vk12_features);

        let device_create_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(device_queue_create_infos)
            .enabled_extension_names(device_extensions)
            .push_next(&mut physical_device_features);

        let device = unsafe {
            instance
                .create_device(physical_device, &device_create_info, None)
                .unwrap()
        };

        let swapchain_loader = ash::khr::swapchain::Device::new(&instance, &device);

        let allocator_create_info =
            vk_mem::AllocatorCreateInfo::new(&instance, &device, physical_device);

        let allocator = unsafe { vk_mem::Allocator::new(allocator_create_info).unwrap() };

        let graphics_queue = unsafe { device.get_device_queue(graphics_queue_family_index, 0) };

        let descriptor_pool = Self::create_descriptor_pool(&device, 30);
        let descriptor_set_layouts = Self::create_descriptor_layouts(&device);

        let all_surface_formats = unsafe {
            surface_loader
                .get_physical_device_surface_formats(physical_device, surface)
                .unwrap()
        };

        let surface_format = all_surface_formats
            .into_iter()
            .find(|&format| {
                format.format == vk::Format::B8G8R8A8_SRGB
                    && format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
            })
            .unwrap();

        let window_size = window.inner_size();
        let (swapchain, swapchain_images, swapchain_image_views, swapchain_extent) =
            create_swapchain(
                &device,
                &surface_loader,
                &swapchain_loader,
                physical_device,
                surface,
                surface_format,
                (window_size.width, window_size.height),
                None,
            )
            .unwrap();

        let command_pool = create_command_pool(&device, graphics_queue_family_index);

        let mut skybox_textures = Vec::new();

        let fallback_skybox = Texture::from_skybox_data(
            &device,
            &allocator,
            graphics_queue,
            command_pool,
            vk::Filter::NEAREST,
            &SkyboxImageData {
                width: 2,
                height: 2,
                top: &[
                    255, 0, 255, 255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 0, 255, 255,
                ],
                bottom: &[
                    255, 0, 255, 255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 0, 255, 255,
                ],
                left: &[
                    255, 0, 255, 255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 0, 255, 255,
                ],
                right: &[
                    255, 0, 255, 255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 0, 255, 255,
                ],
                front: &[
                    255, 0, 255, 255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 0, 255, 255,
                ],
                back: &[
                    255, 0, 255, 255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 0, 255, 255,
                ],
            },
        );

        skybox_textures.push(fallback_skybox);

        let render_frames = Self::create_render_frames(
            &device,
            &allocator,
            graphics_queue,
            descriptor_pool,
            &descriptor_set_layouts,
            swapchain_extent,
            graphics_queue_family_index,
        );

        let submit_semaphores = create_submit_semaphores(&device, swapchain_images.len());

        let pipeline_layout_3d = Self::create_3d_pipeline_layout(&device, &descriptor_set_layouts);
        let pipeline_layout_2d = Self::create_2d_pipeline_layout(&device, &descriptor_set_layouts);

        let pipeline_objects = RendererPipelineObjects::new(
            &device,
            pipeline_layout_3d,
            pipeline_layout_2d,
            surface_format,
        );

        let current_frame: usize = 0;
        let should_resize = false;

        let nearest_neighbor_sampler = create_nearest_sampler(&device);

        let fallback_texture = Texture::from_rgba_data(
            &device,
            &allocator,
            graphics_queue,
            command_pool,
            2,
            2,
            &[
                255, 0, 255, 255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 0, 255, 255,
            ],
        );

        let white_image = Texture::from_rgba_data(
            &device,
            &allocator,
            graphics_queue,
            command_pool,
            1,
            1,
            &[255, 255, 255, 255],
        );

        let mesh_buffers = Vec::new();
        let mut textures = Vec::new();
        let mut materials = Vec::new();

        let fallback_texture = TextureDescriptors::create_texture(
            &device,
            &descriptor_set_layouts,
            descriptor_pool,
            fallback_texture,
            Some(nearest_neighbor_sampler),
        );

        let white_texture = TextureDescriptors::create_texture(
            &device,
            &descriptor_set_layouts,
            descriptor_pool,
            white_image,
            Some(nearest_neighbor_sampler),
        );

        textures.push(fallback_texture);
        textures.push(white_texture);

        let base_material = MaterialDescriptor::new(
            &device,
            &allocator,
            graphics_queue,
            command_pool,
            descriptor_pool,
            &descriptor_set_layouts,
            Vec2::splat(0.5),
            32.0,
        );

        materials.push(base_material);

        let glyph_atlas = GlyphAtlas::new(1024, 1024);

        Self {
            entry,
            instance,
            surface,
            surface_format,
            physical_device,
            device,
            graphics_queue_family_index,
            graphics_queue,
            swapchain,
            swapchain_images,
            swapchain_image_views,
            swapchain_extent,
            should_resize,
            command_pool,
            render_frames,
            submit_semaphores,
            current_frame,
            pipeline_layout_3d,
            pipeline_layout_2d,
            allocator,
            descriptor_set_layouts,
            descriptor_pool,
            textures,
            material_descriptors: materials,
            mesh_buffers,
            skybox_textures,
            current_skybox: None,
            pipeline_objects,
            surface_loader,
            swapchain_loader,
            glyph_atlas,
        }
    }

    fn update_global_descriptors(&mut self, scene: &RenderScene) {
        let current_frame = &mut self.render_frames[self.current_frame % MAX_FRAMES];
        let camera_buffer_allocation = current_frame.per_frame_descriptor_data.camera_buffer.1;
        let scene_buffer_allocation = current_frame.per_frame_descriptor_data.scene_buffer.1;
        let global2d_buffer_allocation = current_frame.per_frame_descriptor_data.global2d_buffer.1;

        let aspect_ratio = self.swapchain_extent.width as f32 / self.swapchain_extent.height as f32;

        let (proj_3d, view_3d) = scene.camera.calc_perspective_matrices(aspect_ratio);

        let corners = scene
            .camera
            .calc_frustrum_corners(aspect_ratio, 30.0, 500.0);
        let mut frustrum_avg = Vec3::ZERO;

        for corner in corners.iter() {
            frustrum_avg += corner
        }

        frustrum_avg /= 8.0;

        let lighting = &scene.lighting;
        let camera_position = scene.camera.position;

        let light_translation = lighting.sun_direction + frustrum_avg;

        let light_rotation = Quat::look_at_rh(light_translation, frustrum_avg, Vec3::Y).inverse();

        let light_view =
            Mat4::from_rotation_translation(light_rotation, light_translation).inverse();

        let light_view_mat3 = Mat3::from_mat4(light_view);

        let mut min = Vec3::splat(0.0);
        let mut max = Vec3::splat(0.0);

        for corner in corners.iter() {
            let lsc = light_view_mat3 * corner;
            min = min.min(lsc);
            max = max.max(lsc);
        }

        min.x -= 200.0;
        max.x += 200.0;
        min.y -= 200.0;
        max.y += 200.0;

        let z_mult = 10.0;

        if min.z < 0.0 {
            min.z *= z_mult
        } else {
            min.z /= z_mult
        }

        if max.z < 0.0 {
            max.z /= z_mult
        } else {
            max.z *= z_mult
        }

        let mut light_proj = Mat4::orthographic_rh(min.x, max.x, min.y, max.y, min.z, max.z);
        light_proj.y_axis *= vec4(1.0, -1.0, 1.0, 1.0);

        let camera_ubo = Global3DUniform {
            proj: proj_3d,
            view: view_3d,
            camera_position: vec4(camera_position.x, camera_position.y, camera_position.z, 0.0),
            light_proj,
            light_view,
        };

        let scene_ubo = SceneUniform {
            sun_direction: vec4(
                lighting.sun_direction.x,
                lighting.sun_direction.y,
                lighting.sun_direction.z,
                0.0,
            ),
            sun_color: vec4(
                lighting.sun_color.x,
                lighting.sun_color.y,
                lighting.sun_color.z,
                lighting.sun_power,
            ),
            ambient_color: vec4(
                lighting.ambient_color.x,
                lighting.ambient_color.y,
                lighting.ambient_color.z,
                0.0,
            ),
        };

        let mut proj_2d = Mat4::orthographic_rh(
            0.0,
            self.swapchain_extent.width as f32,
            0.0,
            self.swapchain_extent.height as f32,
            0.0,
            1.0,
        );

        let global2d_ubo = Global2DUniform {
            proj: proj_2d,
            view: Mat4::IDENTITY,
        };

        let camera_alloc_info = self
            .allocator
            .get_allocation_info(&camera_buffer_allocation);

        let scene_alloc_info = self.allocator.get_allocation_info(&scene_buffer_allocation);

        let global2d_alloc_info = self
            .allocator
            .get_allocation_info(&global2d_buffer_allocation);

        unsafe {
            std::ptr::copy_nonoverlapping(&camera_ubo, camera_alloc_info.mapped_data.cast(), 1);
            std::ptr::copy_nonoverlapping(&scene_ubo, scene_alloc_info.mapped_data.cast(), 1);
            std::ptr::copy_nonoverlapping(&global2d_ubo, global2d_alloc_info.mapped_data.cast(), 1);
        };

        if current_frame.per_frame_descriptor_data.skybox_dirty {
            current_frame.per_frame_descriptor_data.update_skybox(
                &self.device,
                &self.skybox_textures[scene.lighting.skybox_id as usize],
            );
            current_frame.per_frame_descriptor_data.skybox_dirty = false;
        }
    }

    fn set_global_descriptor_dirty(&mut self) {
        for render_frame in self.render_frames.iter_mut() {
            render_frame.per_frame_descriptor_data.skybox_dirty = true;
        }
    }

    fn update_vertex2d_buffer(
        &mut self,
        vertex2d_buffer: &(vk::Buffer, vk_mem::Allocation),
        vertex2d_index_buffer: &(vk::Buffer, vk_mem::Allocation),
        scene: &RenderScene,
    ) -> u32 {
        if !scene.ui_dirty {
            return scene.ui.len() as u32 * 6;
        }

        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for ui_frame in scene.ui.iter() {
            ui_frame.push_verts(&mut vertices, &mut indices);
        }

        let vertex_alloc_info = self.allocator.get_allocation_info(&vertex2d_buffer.1);
        let index_alloc_info = self.allocator.get_allocation_info(&vertex2d_index_buffer.1);

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

        indices.len() as u32
    }

    fn update_instance_buffer(
        &mut self,
        instance_buffer: &(vk::Buffer, vk_mem::Allocation),
        scene: &RenderScene,
    ) -> Vec<MeshBatch> {
        let mut meshes = scene.meshes.clone();

        let aspect_ratio = self.swapchain_extent.width as f32 / self.swapchain_extent.height as f32;
        let (proj, view) = scene.camera.calc_perspective_matrices(aspect_ratio);
        let proj_view = proj * view;

        meshes.sort_unstable_by_key(|m| {
            let opacity = m.opacity;
            let is_opaque = opacity == 1.0;
            let depth = if is_opaque {
                0
            } else {
                let model = proj_view * Vec4::new(m.position.x, m.position.y, m.position.z, 1.0);
                let depth = model.z / model.w;
                (depth * 100_000_000_000.0).round() as u32
            };

            (!is_opaque, depth, m.material_id, m.mesh_id)
        });

        let mesh_count = meshes.len().min(MAX_INSTANCE_COUNT);

        let mut instances: Vec<InstanceVertex> = Vec::new();
        let mut batch_infos: Vec<MeshBatch> = Vec::new();

        let mut start = 0;

        while start < mesh_count {
            let is_opaque = meshes[start].opacity == 1.0;
            let depth = if is_opaque {
                0.0
            } else {
                let model = proj_view
                    * Vec4::new(
                        meshes[start].position.x,
                        meshes[start].position.y,
                        meshes[start].position.z,
                        1.0,
                    );
                model.w
            };

            let key = (meshes[start].material_id, meshes[start].mesh_id, depth);

            {
                let mesh = &meshes[start];
                instances.push(InstanceVertex::new(
                    mesh.position,
                    mesh.orientation,
                    mesh.size,
                    mesh.color,
                    mesh.opacity,
                ));
            }

            let mut end = start + 1;

            while end < mesh_count {
                let is_opaque = meshes[end].opacity == 1.0;
                let depth = if is_opaque {
                    0.0
                } else {
                    let model = proj_view
                        * Vec4::new(
                            meshes[end].position.x,
                            meshes[end].position.y,
                            meshes[end].position.z,
                            1.0,
                        );
                    model.w
                };

                let new_key = (meshes[end].material_id, meshes[end].mesh_id, depth);

                if new_key != key {
                    break;
                }

                {
                    let mesh = &meshes[end];
                    instances.push(InstanceVertex::new(
                        mesh.position,
                        mesh.orientation,
                        mesh.size,
                        mesh.color,
                        mesh.opacity,
                    ));
                }

                end += 1;
            }

            batch_infos.push(MeshBatch {
                mesh_id: key.1,
                material_id: key.0,
                instance_offset: start as u64 * mem::size_of::<InstanceVertex>() as u64,
                instance_count: (end - start) as u32,
                is_opaque,
            });

            start = end;
        }

        let alloc_info = self.allocator.get_allocation_info(&instance_buffer.1);

        unsafe {
            std::ptr::copy_nonoverlapping(
                instances.as_ptr(),
                alloc_info.mapped_data.cast(),
                instances.len().min(MAX_INSTANCE_COUNT),
            );
        }

        batch_infos
    }

    fn draw_shadow_map(
        &mut self,
        batch_info: &[MeshBatch],
        command_buffer: vk::CommandBuffer,
        instance_buffer: vk::Buffer,
        shadow_per_frame_descriptor_set: vk::DescriptorSet,
    ) {
        unsafe {
            self.device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_objects.shadow_graphics_pipeline,
            );

            let shadow_descriptor_sets = [
                shadow_per_frame_descriptor_set,
                self.textures[1].descriptor_set,
            ];

            self.device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout_3d,
                0,
                &shadow_descriptor_sets,
                &[],
            );

            for batch in batch_info.iter() {
                // dont render shadows for transparent objects
                // opaque objects are already sorted to be before transparent objects
                if !batch.is_opaque {
                    break;
                }

                let mesh_buffer = self.get_mesh_buffer(batch.mesh_id);

                self.device.cmd_bind_vertex_buffers(
                    command_buffer,
                    0,
                    &[mesh_buffer.vertex_buffer.0, instance_buffer],
                    &[0, batch.instance_offset],
                );

                self.device.cmd_bind_index_buffer(
                    command_buffer,
                    mesh_buffer.index_buffer.0,
                    0,
                    vk::IndexType::UINT16,
                );

                self.device.cmd_draw_indexed(
                    command_buffer,
                    mesh_buffer.index_count,
                    batch.instance_count,
                    0,
                    0,
                    0,
                );
            }
        }
    }

    fn draw_skybox(
        &mut self,
        command_buffer: vk::CommandBuffer,
        main_per_frame_descriptor_set: vk::DescriptorSet,
    ) {
        unsafe {
            let main_descriptor_sets = [
                main_per_frame_descriptor_set,
                self.textures[0].descriptor_set,
                self.material_descriptors[0].descriptor_set,
            ];

            self.device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout_3d,
                0,
                &main_descriptor_sets,
                &[],
            );

            self.device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_objects.skybox_graphics_pipeline,
            );

            self.device.cmd_bind_vertex_buffers(
                command_buffer,
                0,
                &[self.mesh_buffers[0].vertex_buffer.0],
                &[0],
            );

            self.device.cmd_bind_index_buffer(
                command_buffer,
                self.mesh_buffers[0].index_buffer.0,
                0,
                vk::IndexType::UINT16,
            );

            self.device.cmd_draw_indexed(
                command_buffer,
                self.mesh_buffers[0].index_count,
                1,
                0,
                0,
                0,
            );
        }
    }

    fn draw_main_scene(
        &mut self,
        batch_info: &[MeshBatch],
        command_buffer: vk::CommandBuffer,
        instance_buffer: vk::Buffer,
    ) {
        unsafe {
            let mut last_material = None;
            let mut is_opaque = None;

            for batch in batch_info.iter() {
                if is_opaque != Some(batch.is_opaque) {
                    is_opaque = Some(batch.is_opaque);

                    let pipeline = if batch.is_opaque {
                        self.pipeline_objects.main_graphics_pipeline
                    } else {
                        self.pipeline_objects.main_transparent_graphics_pipeline
                    };

                    self.device.cmd_bind_pipeline(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        pipeline,
                    );
                }

                if last_material != Some(batch.material_id) {
                    last_material = Some(batch.material_id);

                    let descriptor_sets =
                        [self.textures[batch.material_id as usize].descriptor_set];

                    self.device.cmd_bind_descriptor_sets(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        self.pipeline_layout_3d,
                        1,
                        &descriptor_sets,
                        &[],
                    );
                }

                let mesh_buffer = self.get_mesh_buffer(batch.mesh_id);

                self.device.cmd_bind_vertex_buffers(
                    command_buffer,
                    0,
                    &[mesh_buffer.vertex_buffer.0, instance_buffer],
                    &[0, batch.instance_offset],
                );

                self.device.cmd_bind_index_buffer(
                    command_buffer,
                    mesh_buffer.index_buffer.0,
                    0,
                    vk::IndexType::UINT16,
                );

                self.device.cmd_draw_indexed(
                    command_buffer,
                    mesh_buffer.index_count,
                    batch.instance_count,
                    0,
                    0,
                    0,
                );
            }
        }
    }

    fn draw_2d(
        &mut self,
        command_buffer: vk::CommandBuffer,
        global2d_descriptor_set: vk::DescriptorSet,
        vertex_buffer: vk::Buffer,
        index_buffer: vk::Buffer,
        count: u32,
    ) {
        unsafe {
            let descriptor_sets = [global2d_descriptor_set, self.textures[1].descriptor_set];

            self.device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout_2d,
                0,
                &descriptor_sets,
                &[],
            );

            self.device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_objects.main_2d_graphics_pipeline,
            );

            self.device
                .cmd_bind_vertex_buffers(command_buffer, 0, &[vertex_buffer], &[0]);

            self.device.cmd_bind_index_buffer(
                command_buffer,
                index_buffer,
                0,
                vk::IndexType::UINT16,
            );

            self.device
                .cmd_draw_indexed(command_buffer, count, 1, 0, 0, 0);
        }
    }

    pub fn draw(&mut self, scene: &RenderScene) {
        if self.should_resize {
            // self.handle_resize();
            return;
        }

        let current_frame_index = self.current_frame % MAX_FRAMES;
        let current_frame = &mut self.render_frames[current_frame_index];
        let command_pool = current_frame.command_pool;
        let command_buffer = current_frame.command_buffer;
        let swapchain_semaphore = current_frame.swapchain_semaphore;
        let in_flight_fence = current_frame.in_flight_fence;
        let main_per_frame_descriptor_set = current_frame
            .per_frame_descriptor_data
            .main_pass_descriptor_set;
        let shadow_per_frame_descriptor_set = current_frame
            .per_frame_descriptor_data
            .shadow_pass_descriptor_set;
        let global2d_descriptor_set = current_frame
            .per_frame_descriptor_data
            .main_2d_pass_descriptor_set;
        let depth_image_view = current_frame.depth_image_view;
        let shadow_image = current_frame.shadow_map;
        let shadow_image_view = current_frame.shadow_map_view;
        let instance_buffer = current_frame.instance_buffer;
        let vertex2d_buffer = current_frame.vertex2d_buffer;
        let vertex2d_index_buffer = current_frame.vertex2d_index_buffer;

        unsafe {
            self.device
                .wait_for_fences(&[in_flight_fence], true, 1000000000)
                .unwrap();

            let (image_index, should_recreate) = match self.swapchain_loader.acquire_next_image(
                self.swapchain,
                u64::MAX,
                swapchain_semaphore,
                vk::Fence::null(),
            ) {
                Ok(r) => r,
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => (0, true),
                Err(err) => panic!("{}", err),
            };

            if should_recreate {
                self.should_resize = true;
                return;
            }

            self.device.reset_fences(&[in_flight_fence]).unwrap();

            if self.current_skybox != Some(scene.lighting.skybox_id) {
                self.current_skybox = Some(scene.lighting.skybox_id);
                self.set_global_descriptor_dirty();
            }

            self.update_global_descriptors(scene);

            let vertex2d_count =
                self.update_vertex2d_buffer(&vertex2d_buffer, &vertex2d_index_buffer, scene);

            let batch_info = self.update_instance_buffer(&instance_buffer, scene);

            let submit_semaphore = self.submit_semaphores[image_index as usize];

            let swapchain_image = self.swapchain_images[image_index as usize];
            let swapchain_image_view = self.swapchain_image_views[image_index as usize];

            self.device
                .reset_command_pool(command_pool, vk::CommandPoolResetFlags::empty())
                .unwrap();

            let command_buffer_being_info = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

            self.device
                .begin_command_buffer(command_buffer, &command_buffer_being_info)
                .unwrap();

            let shadow_depth_attachment = vk::RenderingAttachmentInfo::default()
                .image_view(shadow_image_view)
                .image_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .clear_value(vk::ClearValue {
                    depth_stencil: vk::ClearDepthStencilValue {
                        depth: 0.0,
                        stencil: 0,
                    },
                });

            let shadow_render_area = vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: vk::Extent2D {
                    width: SHADOW_MAP_RESOLUTION,
                    height: SHADOW_MAP_RESOLUTION,
                },
            };

            let shadow_rendering_info = vk::RenderingInfo::default()
                .depth_attachment(&shadow_depth_attachment)
                .render_area(shadow_render_area)
                .layer_count(1);

            transition_images(
                &self.device,
                command_buffer,
                &[ImageTransition {
                    image: shadow_image.0,
                    current_layout: vk::ImageLayout::UNDEFINED,
                    new_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                    src_stage: vk::PipelineStageFlags2::TOP_OF_PIPE,
                    src_access: vk::AccessFlags2::empty(),
                    dst_stage: vk::PipelineStageFlags2::EARLY_FRAGMENT_TESTS,
                    dst_access: vk::AccessFlags2::DEPTH_STENCIL_ATTACHMENT_WRITE,
                    aspect_mask: vk::ImageAspectFlags::DEPTH,
                }
                .as_barrier()],
            );

            self.device
                .cmd_begin_rendering(command_buffer, &shadow_rendering_info);

            self.device
                .cmd_set_scissor(command_buffer, 0, &[shadow_render_area]);

            self.device.cmd_set_viewport(
                command_buffer,
                0,
                &[vk::Viewport {
                    x: 0.0,
                    y: 0.0,
                    width: SHADOW_MAP_RESOLUTION as f32,
                    height: SHADOW_MAP_RESOLUTION as f32,
                    min_depth: 0.0,
                    max_depth: 1.0,
                }],
            );

            self.draw_shadow_map(
                &batch_info,
                command_buffer,
                instance_buffer.0,
                shadow_per_frame_descriptor_set,
            );

            self.device.cmd_end_rendering(command_buffer);

            let main_rendering_attachments = &[vk::RenderingAttachmentInfo::default()
                .image_view(swapchain_image_view)
                .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .clear_value(vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.0, 0.0, 0.0, 1.0],
                    },
                })];

            let main_depth_attachment = vk::RenderingAttachmentInfo::default()
                .image_view(depth_image_view)
                .image_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .clear_value(vk::ClearValue {
                    depth_stencil: vk::ClearDepthStencilValue {
                        depth: 0.0,
                        stencil: 0,
                    },
                });

            let main_render_area = vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: self.swapchain_extent,
            };

            let main_rendering_info = vk::RenderingInfo::default()
                .color_attachments(main_rendering_attachments)
                .depth_attachment(&main_depth_attachment)
                .render_area(main_render_area)
                .layer_count(1);

            // transition_image(
            //     &self.device,
            //     command_buffer,
            //     swapchain_image,
            //     vk::ImageLayout::UNDEFINED,
            //     vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            //     vk::ImageAspectFlags::COLOR,
            // );

            transition_images(
                &self.device,
                command_buffer,
                &[
                    ImageTransition {
                        image: swapchain_image,
                        current_layout: vk::ImageLayout::UNDEFINED,
                        new_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                        src_stage: vk::PipelineStageFlags2::NONE,
                        src_access: vk::AccessFlags2::NONE,
                        dst_stage: vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                        dst_access: vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                    }
                    .as_barrier(),
                    ImageTransition {
                        image: shadow_image.0,
                        current_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                        new_layout: vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL,
                        src_stage: vk::PipelineStageFlags2::EARLY_FRAGMENT_TESTS,
                        src_access: vk::AccessFlags2::DEPTH_STENCIL_ATTACHMENT_WRITE,
                        dst_stage: vk::PipelineStageFlags2::FRAGMENT_SHADER,
                        dst_access: vk::AccessFlags2::SHADER_SAMPLED_READ,
                        aspect_mask: vk::ImageAspectFlags::DEPTH,
                    }
                    .as_barrier(),
                ],
            );

            self.device
                .cmd_begin_rendering(command_buffer, &main_rendering_info);

            self.device
                .cmd_set_scissor(command_buffer, 0, &[main_render_area]);

            self.device.cmd_set_viewport(
                command_buffer,
                0,
                &[vk::Viewport {
                    x: 0.0,
                    y: 0.0,
                    width: main_render_area.extent.width as f32,
                    height: main_render_area.extent.height as f32,
                    min_depth: 0.0,
                    max_depth: 1.0,
                }],
            );

            self.draw_skybox(command_buffer, main_per_frame_descriptor_set);
            self.draw_main_scene(&batch_info, command_buffer, instance_buffer.0);
            self.draw_2d(
                command_buffer,
                global2d_descriptor_set,
                vertex2d_buffer.0,
                vertex2d_index_buffer.0,
                vertex2d_count,
            );

            self.device.cmd_end_rendering(command_buffer);

            transition_images(
                &self.device,
                command_buffer,
                &[ImageTransition {
                    image: swapchain_image,
                    current_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                    new_layout: vk::ImageLayout::PRESENT_SRC_KHR,
                    src_stage: vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                    src_access: vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
                    dst_stage: vk::PipelineStageFlags2::NONE,
                    dst_access: vk::AccessFlags2::NONE,
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                }
                .as_barrier()],
            );

            self.device.end_command_buffer(command_buffer).unwrap();

            let command_buffer_submit_info = &[vk::CommandBufferSubmitInfo::default()
                .command_buffer(command_buffer)
                .device_mask(0)];

            let wait_info = &[vk::SemaphoreSubmitInfo::default()
                .semaphore(swapchain_semaphore)
                .stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT_KHR)
                .device_index(0)
                .value(0)];

            let signal_info = &[vk::SemaphoreSubmitInfo::default()
                .semaphore(submit_semaphore)
                .stage_mask(vk::PipelineStageFlags2::ALL_GRAPHICS)
                .device_index(0)
                .value(0)];

            let submit_info = &[vk::SubmitInfo2::default()
                .wait_semaphore_infos(wait_info)
                .signal_semaphore_infos(signal_info)
                .command_buffer_infos(command_buffer_submit_info)];

            self.device
                .queue_submit2(self.graphics_queue, submit_info, in_flight_fence)
                .unwrap();

            let swapchains = &[self.swapchain];
            let wait_semaphores = &[submit_semaphore];
            let image_indices = &[image_index];

            let present_info = vk::PresentInfoKHR::default()
                .swapchains(swapchains)
                .wait_semaphores(wait_semaphores)
                .image_indices(image_indices);

            let should_recreate = match self
                .swapchain_loader
                .queue_present(self.graphics_queue, &present_info)
            {
                Ok(r) => r,
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => true,
                Err(err) => panic!("{}", err),
            };

            if should_recreate {
                self.should_resize = true;
            }
        }

        self.current_frame += 1;
    }

    pub fn handle_resize(&mut self, window_size: (u32, u32)) {
        unsafe { self.device.device_wait_idle().unwrap() };

        let (swapchain, swapchain_images, swapchain_image_views, swapchain_extent) =
            match create_swapchain(
                &self.device,
                &self.surface_loader,
                &self.swapchain_loader,
                self.physical_device,
                self.surface,
                self.surface_format,
                window_size,
                Some(self.swapchain),
            ) {
                Ok(r) => r,
                Err(vk::Result::ERROR_NATIVE_WINDOW_IN_USE_KHR) => {
                    return;
                }
                Err(err) => panic!("{}", err),
            };

        for &image_view in self.swapchain_image_views.iter() {
            unsafe { self.device.destroy_image_view(image_view, None) };
        }

        unsafe {
            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None)
        };

        self.swapchain = swapchain;
        self.swapchain_images = swapchain_images;
        self.swapchain_image_views = swapchain_image_views;
        self.swapchain_extent = swapchain_extent;
        self.should_resize = false;

        for frame in self.render_frames.iter_mut() {
            frame.handle_resize(
                &self.device,
                &self.allocator,
                self.command_pool,
                self.graphics_queue,
                swapchain_extent,
            );
        }
    }

    pub fn load_mesh(&mut self, vertices: &[MeshVertex], indices: &[u16]) -> MeshHandle {
        let mesh = MeshBuffer::allocate_mesh(
            &self.device,
            &self.allocator,
            self.graphics_queue,
            self.command_pool,
            vertices,
            indices,
        );

        self.mesh_buffers.push(mesh);

        MeshHandle((self.mesh_buffers.len() - 1) as u32)
    }

    pub fn load_rgba_texture(&mut self, width: u32, height: u32, data: &[u8]) -> u32 {
        let mut texture = Texture::from_rgba_data(
            &self.device,
            &self.allocator,
            self.graphics_queue,
            self.command_pool,
            width,
            height,
            data,
        );

        texture.sampler = Some(create_nearest_sampler(&self.device));

        let texture = TextureDescriptors::create_texture(
            &self.device,
            &self.descriptor_set_layouts,
            self.descriptor_pool,
            texture,
            None,
        );

        self.textures.push(texture);

        (self.textures.len() - 1) as u32
    }

    pub fn load_skybox(
        &mut self,
        sampler_filter: vk::Filter,
        skybox_data: &SkyboxImageData,
    ) -> u32 {
        let skybox_texture = Texture::from_skybox_data(
            &self.device,
            &self.allocator,
            self.graphics_queue,
            self.command_pool,
            sampler_filter,
            skybox_data,
        );

        self.skybox_textures.push(skybox_texture);
        (self.skybox_textures.len() - 1) as u32
    }

    fn create_3d_pipeline_layout(
        device: &ash::Device,
        descriptor_set_layouts: &DescriptorSetLayouts,
    ) -> vk::PipelineLayout {
        let layouts = &[
            descriptor_set_layouts.global_3d_layout,
            descriptor_set_layouts.texture_layout,
            descriptor_set_layouts.material_layout,
        ];

        let pipeline_layout_info = vk::PipelineLayoutCreateInfo::default().set_layouts(layouts);

        unsafe {
            device
                .create_pipeline_layout(&pipeline_layout_info, None)
                .unwrap()
        }
    }

    fn create_2d_pipeline_layout(
        device: &ash::Device,
        descriptor_set_layouts: &DescriptorSetLayouts,
    ) -> vk::PipelineLayout {
        let layouts = &[
            descriptor_set_layouts.global_2d_layout,
            descriptor_set_layouts.texture_layout,
        ];

        let pipeline_layout_info = vk::PipelineLayoutCreateInfo::default().set_layouts(layouts);

        unsafe {
            device
                .create_pipeline_layout(&pipeline_layout_info, None)
                .unwrap()
        }
    }

    fn get_mesh_buffer(&self, mesh_id: MeshHandle) -> &MeshBuffer {
        &self.mesh_buffers[mesh_id.0 as usize]
    }

    fn create_descriptor_pool(device: &ash::Device, set_count: u32) -> vk::DescriptorPool {
        let descriptor_pool_sizes: Vec<vk::DescriptorPoolSize> = DESCRIPTOR_RATIOS
            .iter()
            .map(|(ty, ratio)| {
                vk::DescriptorPoolSize::default()
                    .ty(*ty)
                    .descriptor_count(ratio * set_count * MAX_FRAMES as u32)
            })
            .collect();

        let descriptor_pool_info = vk::DescriptorPoolCreateInfo::default()
            .max_sets(set_count * MAX_FRAMES as u32)
            .pool_sizes(&descriptor_pool_sizes);

        let descriptor_pool = unsafe {
            device
                .create_descriptor_pool(&descriptor_pool_info, None)
                .unwrap()
        };

        descriptor_pool
    }

    fn create_descriptor_layouts(device: &ash::Device) -> DescriptorSetLayouts {
        let global_3d_bindings = [
            vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT),
            vk::DescriptorSetLayoutBinding::default()
                .binding(1)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT),
            vk::DescriptorSetLayoutBinding::default()
                .binding(2)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT),
            vk::DescriptorSetLayoutBinding::default()
                .binding(3)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT),
        ];

        let global_3d_layout_info =
            vk::DescriptorSetLayoutCreateInfo::default().bindings(&global_3d_bindings);

        let global_3d_layout = unsafe {
            device
                .create_descriptor_set_layout(&global_3d_layout_info, None)
                .unwrap()
        };

        let texture_bindings = [vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)];

        let texture_layout_info =
            vk::DescriptorSetLayoutCreateInfo::default().bindings(&texture_bindings);

        let texture_layout = unsafe {
            device
                .create_descriptor_set_layout(&texture_layout_info, None)
                .unwrap()
        };

        let material_bindings = [vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)];

        let material_layout_info =
            vk::DescriptorSetLayoutCreateInfo::default().bindings(&material_bindings);

        let material_layout = unsafe {
            device
                .create_descriptor_set_layout(&material_layout_info, None)
                .unwrap()
        };

        let global_2d_bindings = [vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)];

        let global_2d_layout_info =
            vk::DescriptorSetLayoutCreateInfo::default().bindings(&global_2d_bindings);

        let global_2d_layout = unsafe {
            device
                .create_descriptor_set_layout(&global_2d_layout_info, None)
                .unwrap()
        };

        DescriptorSetLayouts {
            global_2d_layout,
            global_3d_layout,
            texture_layout,
            material_layout,
        }
    }

    fn create_render_frames(
        device: &ash::Device,
        allocator: &vk_mem::Allocator,
        queue: vk::Queue,
        descriptor_pool: vk::DescriptorPool,
        descriptor_layouts: &DescriptorSetLayouts,
        window_extent: vk::Extent2D,
        queue_family_index: u32,
    ) -> Vec<RenderFrame> {
        let frames: Vec<RenderFrame> = (0..MAX_FRAMES)
            .map(|_i| -> RenderFrame {
                RenderFrame::new(
                    device,
                    allocator,
                    queue,
                    descriptor_pool,
                    descriptor_layouts,
                    window_extent,
                    queue_family_index,
                )
            })
            .collect();

        frames
    }
}

impl Drop for VulkanContext {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.device_wait_idle();

            for skybox_texture in self.skybox_textures.iter_mut() {
                skybox_texture.destroy(&self.device, &self.allocator);
            }

            for mesh_buffer in self.mesh_buffers.iter_mut() {
                mesh_buffer.destroy(&self.allocator);
            }

            for texture in self.textures.iter_mut() {
                texture.destroy(&self.device, &self.allocator);
            }

            for material in self.material_descriptors.iter_mut() {
                material.destroy(&self.allocator);
            }

            for render_frame in self.render_frames.iter_mut() {
                render_frame.destroy(&self.device, &self.allocator);
            }
        };
    }
}
