use std::{io::Cursor, mem};

use ash::{prelude::VkResult, vk};
use vk_mem::Alloc;

pub type AllocatedImage = (vk::Image, vk_mem::Allocation);
pub type AllocatedBuffer = (vk::Buffer, vk_mem::Allocation);

pub fn destroy_allocated_image(allocator: &vk_mem::Allocator, image: &mut AllocatedImage) {
    unsafe { allocator.destroy_image(image.0, &mut image.1) }
}

pub fn destroy_allocated_buffer(allocator: &vk_mem::Allocator, buffer: &mut AllocatedBuffer) {
    unsafe { allocator.destroy_buffer(buffer.0, &mut buffer.1) }
}

pub fn create_command_pool(
    device: &ash::Device,
    queue_family_index: u32,
) -> VkResult<vk::CommandPool> {
    let command_pool_create_info = vk::CommandPoolCreateInfo::default()
        .queue_family_index(queue_family_index)
        .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);

    unsafe { device.create_command_pool(&command_pool_create_info, None) }
}

pub fn transition_image(
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

pub fn allocate_command_buffer(
    device: &ash::Device,
    command_pool: vk::CommandPool,
    buffer_level: vk::CommandBufferLevel,
) -> VkResult<vk::CommandBuffer> {
    let command_buffers = unsafe {
        device.allocate_command_buffers(
            &vk::CommandBufferAllocateInfo::default()
                .command_pool(command_pool)
                .command_buffer_count(1)
                .level(buffer_level),
        )?
    };

    Ok(command_buffers[0])
}
pub fn allocate_command_buffers(
    device: &ash::Device,
    command_pool: vk::CommandPool,
    buffer_count: u32,
    buffer_level: vk::CommandBufferLevel,
) -> VkResult<Vec<vk::CommandBuffer>> {
    let command_buffers = unsafe {
        device.allocate_command_buffers(
            &vk::CommandBufferAllocateInfo::default()
                .command_pool(command_pool)
                .command_buffer_count(buffer_count)
                .level(buffer_level),
        )?
    };

    Ok(command_buffers)
}

pub fn create_uniform_buffer<T>(allocator: &vk_mem::Allocator) -> VkResult<AllocatedBuffer> {
    let buffer_info = vk::BufferCreateInfo::default()
        .usage(vk::BufferUsageFlags::UNIFORM_BUFFER)
        .size(mem::size_of::<T>() as u64);

    let alloc_info = vk_mem::AllocationCreateInfo {
        flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE
            | vk_mem::AllocationCreateFlags::MAPPED,
        usage: vk_mem::MemoryUsage::AutoPreferHost,

        ..Default::default()
    };

    unsafe { allocator.create_buffer(&buffer_info, &alloc_info) }
}

pub fn create_shader_module(device: &ash::Device, data: &[u8]) -> VkResult<vk::ShaderModule> {
    let mut cursor = Cursor::new(data);

    let spv = ash::util::read_spv(&mut cursor).unwrap();
    let create_info = vk::ShaderModuleCreateInfo::default().code(&spv);

    unsafe { device.create_shader_module(&create_info, None) }
}
