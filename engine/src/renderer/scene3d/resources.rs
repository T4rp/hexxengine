use std::mem;

use ash::{prelude::VkResult, vk};
use glam::{Mat3, Mat4, Quat, Vec3, Vec4};
use vk_mem::Alloc;

use crate::{
    renderer::{
        renderer::{MaterialDescriptor, MeshBuffer, MeshHandle, TextureDescriptors},
        scene3d::{
            CameraUniform3d, InstanceVertex, MAX_INSTANCE_COUNT, SHADOW_MAP_RESOLUTION,
            Scene3dUniform,
        },
        textures::Texture,
        vk_deletion_queue::{self, VulkanDeletionQueue},
        vkutils,
    },
    scene::RenderScene,
};

#[derive(Debug)]
pub struct MeshBatch {
    pub mesh_id: MeshHandle,
    pub material_id: u32,
    pub instance_offset: u64,
    pub instance_count: u32,
    pub is_opaque: bool,
}

pub struct Resources {
    pub main_pass_descriptor_set: vk::DescriptorSet,
    pub shadow_pass_descriptor_set: vk::DescriptorSet,

    pub shadow_map_sampler: vk::Sampler,
    pub skybox_outdated: bool,

    pub camera_uniform_buffer: (vk::Buffer, vk_mem::Allocation),
    pub scene_uniform_buffer: (vk::Buffer, vk_mem::Allocation),

    pub instance_buffer: (vk::Buffer, vk_mem::Allocation),

    pub depth_image_view: vk::ImageView,
    pub depth_image: vkutils::AllocatedImage,

    pub shadow_map_image: vkutils::AllocatedImage,
    pub shadow_map_image_view: vk::ImageView,
}

impl Resources {
    pub fn new(
        device: &ash::Device,
        allocator: &vk_mem::Allocator,
        command_pool: vk::CommandPool,
        queue: vk::Queue,
        _queue_family_index: u32,
        descriptor_pool: vk::DescriptorPool,
        scene_descriptor_layout: vk::DescriptorSetLayout,
        window_extent: vk::Extent2D,
    ) -> VkResult<Self> {
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
        let shadow_map_image_view = Self::create_shadow_map_image_view(device, &shadow_map_image)?;
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
            shadow_map_image,
            shadow_map_image_view,
        })
    }

    pub fn destroy(&mut self, _device: &ash::Device, allocator: &vk_mem::Allocator) {
        vkutils::destroy_allocated_buffer(allocator, &mut self.camera_uniform_buffer);
        vkutils::destroy_allocated_buffer(allocator, &mut self.scene_uniform_buffer);
        vkutils::destroy_allocated_buffer(allocator, &mut self.instance_buffer);
        vkutils::destroy_allocated_image(allocator, &mut self.depth_image);
        vkutils::destroy_allocated_image(allocator, &mut self.shadow_map_image);
    }

    pub fn target_resized(
        &mut self,
        device: &ash::Device,
        allocator: &vk_mem::Allocator,
        deletion_queue: &mut VulkanDeletionQueue,
        command_pool: vk::CommandPool,
        queue: vk::Queue,
        window_extent: vk::Extent2D,
    ) -> VkResult<()> {
        deletion_queue.push(vk_deletion_queue::Resource::ImageView(
            self.depth_image_view,
        ));
        deletion_queue.push(vk_deletion_queue::Resource::AllocatedImage(
            self.depth_image,
        ));

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

        Ok(())
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

    pub fn update_uniform_buffers(
        &mut self,
        allocator: &vk_mem::Allocator,
        scene: &RenderScene,
        window_extent: vk::Extent2D,
    ) {
        let camera_buffer_allocation = self.camera_uniform_buffer.1;
        let scene_buffer_allocation = self.scene_uniform_buffer.1;

        let aspect_ratio = window_extent.width as f32 / window_extent.height as f32;
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

        let shadow_snap = 1.0 / SHADOW_MAP_RESOLUTION as f32;

        min /= shadow_snap;
        min = min.floor();
        min *= shadow_snap;

        max /= shadow_snap;
        max = max.floor();
        max *= shadow_snap;

        let mut light_proj = Mat4::orthographic_rh(min.x, max.x, min.y, max.y, min.z, max.z);
        light_proj.y_axis *= Vec4::new(1.0, -1.0, 1.0, 1.0);

        let camera_ubo = CameraUniform3d {
            proj: proj_3d,
            view: view_3d,
            camera_position: Vec4::new(
                camera_position.x,
                camera_position.y,
                camera_position.z,
                0.0,
            ),
            light_proj,
            light_view,
        };

        let scene_ubo = Scene3dUniform {
            sun_direction: Vec4::new(
                lighting.sun_direction.x,
                lighting.sun_direction.y,
                lighting.sun_direction.z,
                0.0,
            ),
            sun_color: Vec4::new(
                lighting.sun_color.x,
                lighting.sun_color.y,
                lighting.sun_color.z,
                lighting.sun_power,
            ),
            ambient_color: Vec4::new(
                lighting.ambient_color.x,
                lighting.ambient_color.y,
                lighting.ambient_color.z,
                0.0,
            ),
        };

        let camera_alloc_info = allocator.get_allocation_info(&camera_buffer_allocation);
        let scene_alloc_info = allocator.get_allocation_info(&scene_buffer_allocation);

        unsafe {
            std::ptr::copy_nonoverlapping(&camera_ubo, camera_alloc_info.mapped_data.cast(), 1);
            std::ptr::copy_nonoverlapping(&scene_ubo, scene_alloc_info.mapped_data.cast(), 1);
        };
    }

    pub fn update_instance_buffer(
        &mut self,
        allocator: &vk_mem::Allocator,
        scene: &RenderScene,
        window_extent: vk::Extent2D,
    ) -> Vec<MeshBatch> {
        let mut meshes = scene.meshes.clone();

        let aspect_ratio = window_extent.width as f32 / window_extent.height as f32;
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

        let alloc_info = allocator.get_allocation_info(&self.instance_buffer.1);

        unsafe {
            std::ptr::copy_nonoverlapping(
                instances.as_ptr(),
                alloc_info.mapped_data.cast(),
                instances.len().min(MAX_INSTANCE_COUNT),
            );
        }

        batch_infos
    }

    pub fn draw_shadow_map(
        &mut self,
        device: &ash::Device,
        command_buffer: vk::CommandBuffer,
        shadow_pipeline: vk::Pipeline,
        pipeline_layout: vk::PipelineLayout,
        meshes: &[MeshBuffer],
        textures: &[TextureDescriptors],
        batch_info: &[MeshBatch],
    ) {
        unsafe {
            device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                shadow_pipeline,
            );

            let shadow_descriptor_sets =
                [self.shadow_pass_descriptor_set, textures[1].descriptor_set];

            device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline_layout,
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

                let mesh_buffer = &meshes[batch.mesh_id.0 as usize];

                device.cmd_bind_vertex_buffers(
                    command_buffer,
                    0,
                    &[mesh_buffer.vertex_buffer.0, self.instance_buffer.0],
                    &[0, batch.instance_offset],
                );

                device.cmd_bind_index_buffer(
                    command_buffer,
                    mesh_buffer.index_buffer.0,
                    0,
                    vk::IndexType::UINT16,
                );

                device.cmd_draw_indexed(
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

    pub fn draw_skybox(
        &mut self,
        device: &ash::Device,
        command_buffer: vk::CommandBuffer,
        skybox_pipeline: vk::Pipeline,
        pipeline_layout: vk::PipelineLayout,
        meshes: &[MeshBuffer],
        textures: &[TextureDescriptors],
        materials: &[MaterialDescriptor],
    ) {
        unsafe {
            let main_descriptor_sets = [
                self.main_pass_descriptor_set,
                textures[0].descriptor_set,
                materials[0].descriptor_set,
            ];

            device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline_layout,
                0,
                &main_descriptor_sets,
                &[],
            );

            device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                skybox_pipeline,
            );

            let cube_mesh = &meshes[0];

            device.cmd_bind_vertex_buffers(command_buffer, 0, &[cube_mesh.vertex_buffer.0], &[0]);

            device.cmd_bind_index_buffer(
                command_buffer,
                cube_mesh.index_buffer.0,
                0,
                vk::IndexType::UINT16,
            );

            device.cmd_draw_indexed(command_buffer, cube_mesh.index_count, 1, 0, 0, 0);
        }
    }

    pub fn draw_scene(
        &mut self,
        device: &ash::Device,
        command_buffer: vk::CommandBuffer,
        opaque_scene_pipeline: vk::Pipeline,
        transparent_scene_pipeline: vk::Pipeline,
        pipeline_layout: vk::PipelineLayout,
        meshes: &[MeshBuffer],
        textures: &[TextureDescriptors],
        _materials: &[MaterialDescriptor],
        batch_info: &[MeshBatch],
    ) {
        unsafe {
            let mut last_material = None;
            let mut is_opaque = None;

            for batch in batch_info.iter() {
                if is_opaque != Some(batch.is_opaque) {
                    is_opaque = Some(batch.is_opaque);

                    let pipeline = if batch.is_opaque {
                        opaque_scene_pipeline
                    } else {
                        transparent_scene_pipeline
                    };

                    device.cmd_bind_pipeline(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        pipeline,
                    );
                }

                if last_material != Some(batch.material_id) {
                    last_material = Some(batch.material_id);

                    let descriptor_sets = [textures[batch.material_id as usize].descriptor_set];

                    device.cmd_bind_descriptor_sets(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        pipeline_layout,
                        1,
                        &descriptor_sets,
                        &[],
                    );
                }

                let mesh_buffer = &meshes[batch.mesh_id.0 as usize];

                device.cmd_bind_vertex_buffers(
                    command_buffer,
                    0,
                    &[mesh_buffer.vertex_buffer.0, self.instance_buffer.0],
                    &[0, batch.instance_offset],
                );

                device.cmd_bind_index_buffer(
                    command_buffer,
                    mesh_buffer.index_buffer.0,
                    0,
                    vk::IndexType::UINT16,
                );

                device.cmd_draw_indexed(
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
