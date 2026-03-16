use std::{array, ptr};

use ash::{prelude::VkResult, vk};
use vk_mem::Alloc;

use crate::renderer::vkutils;

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

pub struct Texture {
    pub image: vkutils::AllocatedImage,
    pub image_view: vk::ImageView,
    pub sampler: Option<vk::Sampler>,
}

impl Texture {
    pub fn from_rgba_data(
        device: &ash::Device,
        allocator: &vk_mem::Allocator,
        queue: vk::Queue,
        command_pool: vk::CommandPool,
        width: u32,
        height: u32,
        data: &[u8],
    ) -> VkResult<Self> {
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
            .usage(vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .samples(vk::SampleCountFlags::TYPE_1)
            .flags(vk::ImageCreateFlags::empty());

        let image_alloc_create_info = vk_mem::AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::AutoPreferDevice,
            ..Default::default()
        };

        let image = unsafe { allocator.create_image(&image_info, &image_alloc_create_info)? };

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

        let image_view = unsafe { device.create_image_view(&image_view_info, None)? };

        let buffer_info = vk::BufferCreateInfo::default()
            .usage(vk::BufferUsageFlags::TRANSFER_SRC)
            .size(width as u64 * height as u64 * 4);

        let create_info = vk_mem::AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::Auto,
            flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE
                | vk_mem::AllocationCreateFlags::MAPPED,
            ..Default::default()
        };

        let mut staging_buffer = unsafe { allocator.create_buffer(&buffer_info, &create_info)? };
        let alloc_info = allocator.get_allocation_info(&staging_buffer.1);

        unsafe {
            ptr::copy_nonoverlapping(data.as_ptr(), alloc_info.mapped_data.cast(), data.len())
        };

        let command_buffers = vkutils::allocate_command_buffers(
            device,
            command_pool,
            1,
            vk::CommandBufferLevel::PRIMARY,
        )?;

        let command_buffer = command_buffers[0];

        let being_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe { device.begin_command_buffer(command_buffer, &being_info)? };

        vkutils::transition_image(
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

        vkutils::transition_image(
            device,
            command_buffer,
            image.0,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            vk::ImageAspectFlags::COLOR,
        );

        unsafe {
            device.end_command_buffer(command_buffer)?;

            device.queue_submit(
                queue,
                &[vk::SubmitInfo::default().command_buffers(&command_buffers)],
                vk::Fence::null(),
            )?;

            device.queue_wait_idle(queue)?;
            allocator.destroy_buffer(staging_buffer.0, &mut staging_buffer.1);
            device.free_command_buffers(command_pool, &[command_buffer]);
        }

        Ok(Self {
            image,
            image_view,
            sampler: None,
        })
    }

    pub fn from_skybox_data(
        device: &ash::Device,
        allocator: &vk_mem::Allocator,
        queue: vk::Queue,
        command_pool: vk::CommandPool,
        sampler_filter: vk::Filter,
        skybox_data: &SkyboxImageData,
    ) -> VkResult<Self> {
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
            .usage(vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .samples(vk::SampleCountFlags::TYPE_1)
            .flags(vk::ImageCreateFlags::CUBE_COMPATIBLE);

        let image_alloc_create_info = vk_mem::AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::AutoPreferDevice,
            ..Default::default()
        };

        let image = unsafe { allocator.create_image(&image_info, &image_alloc_create_info)? };

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

        let image_view = unsafe { device.create_image_view(&image_view_info, None)? };

        let buffer_info = vk::BufferCreateInfo::default()
            .usage(vk::BufferUsageFlags::TRANSFER_SRC)
            .size(all_image_data.len() as u64);

        let create_info = vk_mem::AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::Auto,
            flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE
                | vk_mem::AllocationCreateFlags::MAPPED,
            ..Default::default()
        };

        let mut staging_buffer = unsafe { allocator.create_buffer(&buffer_info, &create_info)? };
        let alloc_info = allocator.get_allocation_info(&staging_buffer.1);

        unsafe {
            ptr::copy_nonoverlapping(
                all_image_data.as_ptr(),
                alloc_info.mapped_data.cast(),
                all_image_data.len(),
            );
        }

        let command_buffers = vkutils::allocate_command_buffers(
            device,
            command_pool,
            1,
            vk::CommandBufferLevel::PRIMARY,
        )?;

        let command_buffer = command_buffers[0];

        let being_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe { device.begin_command_buffer(command_buffer, &being_info)? };

        vkutils::transition_image(
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

        vkutils::transition_image(
            device,
            command_buffer,
            image.0,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            vk::ImageAspectFlags::COLOR,
        );

        unsafe {
            device.end_command_buffer(command_buffer)?;

            device.queue_submit(
                queue,
                &[vk::SubmitInfo::default().command_buffers(&command_buffers)],
                vk::Fence::null(),
            )?;

            device.queue_wait_idle(queue)?;
            allocator.destroy_buffer(staging_buffer.0, &mut staging_buffer.1);
            device.free_command_buffers(command_pool, &[command_buffer]);
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

        let sampler = unsafe { device.create_sampler(&sampler_info, None)? };

        Ok(Texture {
            image,
            image_view,
            sampler: Some(sampler),
        })
    }

    pub fn destroy(&mut self, device: &ash::Device, allocator: &vk_mem::Allocator) {
        unsafe {
            device.destroy_image_view(self.image_view, None);

            if let Some(sampler) = self.sampler {
                device.destroy_sampler(sampler, None);
            }

            allocator.destroy_image(self.image.0, &mut self.image.1);
        }
    }
}
