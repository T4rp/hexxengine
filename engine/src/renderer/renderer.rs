use std::borrow::Cow;
use std::{ffi, mem};

use ash::{khr, vk};
use glam::Vec2;
use vk_mem::Alloc;
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle};
use winit::window::Window;

use crate::renderer::images::{ImageTransition, transition_images};
use crate::renderer::mesh::{MaterialFlags, MaterialUniform, MeshVertex};
use crate::renderer::pipelines::RendererPipelines;
use crate::renderer::scene2d;
use crate::renderer::scene3d::{self, SHADOW_MAP_RESOLUTION};
use crate::renderer::textures::{SkyboxImageData, Texture};
use crate::renderer::vkutils::create_command_pool;
use crate::scene::RenderScene;
use crate::text::{GlyphAtlas, GlyphRenderMode};

const USE_VALIDATION_LAYERS: bool = true;
const MAX_FRAMES: usize = 2;

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

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct MeshHandle(pub u32);
struct RenderFrame {
    command_pool: vk::CommandPool,
    command_buffer: vk::CommandBuffer,
    swapchain_semaphore: vk::Semaphore,
    in_flight_fence: vk::Fence,

    scene2d_resources: scene2d::Resources,
    scene3d_resources: scene3d::Resources,

    skybox_dirty: bool,
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
        glyph_atlas: &GlyphAtlas,
    ) -> Self {
        let command_pool = create_command_pool(device, queue_family_index).unwrap();

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

        let scene3d_resources = scene3d::Resources::new(
            device,
            allocator,
            command_pool,
            queue,
            queue_family_index,
            descriptor_pool,
            descriptor_layouts.global_3d_layout,
            window_extent,
        )
        .unwrap();

        let scene2d_resources = scene2d::Resources::new(
            device,
            allocator,
            descriptor_pool,
            descriptor_layouts.global_2d_layout,
            &glyph_atlas,
        )
        .unwrap();

        RenderFrame {
            command_pool,
            command_buffer,
            swapchain_semaphore,
            in_flight_fence,
            scene2d_resources,
            scene3d_resources,
            skybox_dirty: true,
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

        self.swapchain_semaphore = new_semaphore;

        self.scene3d_resources
            .target_resized(device, allocator, command_pool, queue, window_extent)
            .unwrap();
    }

    fn destroy(&mut self, device: &ash::Device, allocator: &vk_mem::Allocator) {
        self.scene3d_resources.destroy(device, allocator);
        self.scene2d_resources.destroy(device, allocator);
    }
}

struct DescriptorSetLayouts {
    global_3d_layout: vk::DescriptorSetLayout,
    texture_layout: vk::DescriptorSetLayout,
    material_layout: vk::DescriptorSetLayout,
    global_2d_layout: vk::DescriptorSetLayout,
}

pub struct TextureDescriptors {
    pub texture: Texture,
    pub descriptor_set: vk::DescriptorSet,
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

pub struct MeshBuffer {
    pub vertex_buffer: (vk::Buffer, vk_mem::Allocation),
    pub index_buffer: (vk::Buffer, vk_mem::Allocation),
    pub index_count: u32,
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

pub struct MaterialDescriptor {
    pub uniform_buffer: (vk::Buffer, vk_mem::Allocation),
    pub descriptor_set: vk::DescriptorSet,
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
    descriptor_pool: vk::DescriptorPool,
    pipeline_layout_3d: vk::PipelineLayout,
    pipeline_objects: RendererPipelines,
    pipeline_layout_2d: vk::PipelineLayout,

    mesh_buffers: Vec<MeshBuffer>,
    textures: Vec<TextureDescriptors>,
    material_descriptors: Vec<MaterialDescriptor>,
    skybox_textures: Vec<Texture>,
    current_skybox: Option<u32>,

    glyph_atlas: GlyphAtlas,
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
                | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
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

    let mut image_extent = if surface_max_image_extent.width != u32::MAX {
        surface_max_image_extent
    } else {
        vk::Extent2D {
            width: window_size.0,
            height: window_size.1,
        }
    };

    image_extent.height = image_extent.height.max(1);
    image_extent.width = image_extent.width.max(1);

    let create_swapchain_info = vk::SwapchainCreateInfoKHR::default()
        .surface(surface)
        .image_format(surface_format.format)
        .image_color_space(surface_format.color_space)
        .present_mode(vk::PresentModeKHR::FIFO_RELAXED)
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

fn create_nearest_sampler(device: &ash::Device) -> vk::Sampler {
    let sampler_info = vk::SamplerCreateInfo::default()
        .mag_filter(vk::Filter::NEAREST)
        .min_filter(vk::Filter::NEAREST);

    unsafe { device.create_sampler(&sampler_info, None).unwrap() }
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

impl VulkanContext {
    pub fn new(window: &Window) -> Self {
        let glyph_atlas = GlyphAtlas::new(GlyphRenderMode::Sdf, 1024, 1024);

        let raw_window_handle = window.window_handle().unwrap().as_raw();
        let raw_display_handle = window.display_handle().unwrap().as_raw();

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

        let command_pool = create_command_pool(&device, graphics_queue_family_index).unwrap();

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
        )
        .unwrap();

        skybox_textures.push(fallback_skybox);

        let render_frames = Self::create_render_frames(
            &device,
            &allocator,
            graphics_queue,
            descriptor_pool,
            &descriptor_set_layouts,
            swapchain_extent,
            graphics_queue_family_index,
            &glyph_atlas,
        );

        let submit_semaphores = create_submit_semaphores(&device, swapchain_images.len());

        let pipeline_layout_3d = Self::create_3d_pipeline_layout(&device, &descriptor_set_layouts);
        let pipeline_layout_2d = Self::create_2d_pipeline_layout(&device, &descriptor_set_layouts);

        let pipeline_objects = RendererPipelines::new(
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
        )
        .unwrap();

        let white_image = Texture::from_rgba_data(
            &device,
            &allocator,
            graphics_queue,
            command_pool,
            1,
            1,
            &[255, 255, 255, 255],
        )
        .unwrap();

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

    pub fn draw(&mut self, scene: &RenderScene) {
        if self.should_resize {
            return;
        }

        if self.current_skybox != Some(scene.lighting.skybox_id) {
            self.current_skybox = Some(scene.lighting.skybox_id);
            for render_frame in self.render_frames.iter_mut() {
                render_frame.skybox_dirty = true;
            }
        }

        for text_cmd in scene.text_draws.iter() {
            self.glyph_atlas
                .load_glyphs(text_cmd.text.as_ref(), text_cmd.font_height)
                .unwrap();
        }

        for render_frame in self.render_frames.iter_mut() {
            render_frame
                .scene2d_resources
                .mark_glyph_atlas_dirty(&self.glyph_atlas);
        }

        self.glyph_atlas.flush_dirty_region();

        let current_frame_index = self.current_frame % MAX_FRAMES;
        let current_frame = &mut self.render_frames[current_frame_index];
        let command_pool = current_frame.command_pool;
        let command_buffer = current_frame.command_buffer;
        let swapchain_semaphore = current_frame.swapchain_semaphore;
        let in_flight_fence = current_frame.in_flight_fence;

        let scene3d_resources = &mut current_frame.scene3d_resources;
        let scene2d_resources = &mut current_frame.scene2d_resources;

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

            if current_frame.skybox_dirty {
                scene3d_resources.update_skybox(
                    &self.device,
                    &self.skybox_textures[scene.lighting.skybox_id as usize],
                );
                current_frame.skybox_dirty = false;
            }

            scene2d_resources
                .update_atlas_image(
                    &self.device,
                    &self.allocator,
                    self.graphics_queue,
                    current_frame.command_pool,
                    &self.glyph_atlas,
                )
                .unwrap();

            scene3d_resources.update_uniform_buffers(&self.allocator, scene, self.swapchain_extent);
            scene2d_resources.update_uniform_buffers(&self.allocator, self.swapchain_extent);

            scene2d_resources.update_buffers(&self.allocator, scene);

            let batch_info = scene3d_resources.update_instance_buffer(
                &self.allocator,
                scene,
                self.swapchain_extent,
            );

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
                .image_view(scene3d_resources.shadow_map_image_view)
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
                    image: scene3d_resources.shadow_map_image.0,
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

            scene3d_resources.draw_shadow_map(
                &self.device,
                command_buffer,
                self.pipeline_objects.shadow_graphics_pipeline,
                self.pipeline_layout_3d,
                &self.mesh_buffers,
                &self.textures,
                &batch_info,
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
                .image_view(scene3d_resources.depth_image_view)
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
                        image: scene3d_resources.shadow_map_image.0,
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

            scene3d_resources.draw_skybox(
                &self.device,
                command_buffer,
                self.pipeline_objects.skybox_graphics_pipeline,
                self.pipeline_layout_3d,
                &self.mesh_buffers,
                &self.textures,
                &self.material_descriptors,
            );

            scene3d_resources.draw_scene(
                &self.device,
                command_buffer,
                self.pipeline_objects.main_graphics_pipeline,
                self.pipeline_objects.main_transparent_graphics_pipeline,
                self.pipeline_layout_3d,
                &self.mesh_buffers,
                &self.textures,
                &self.material_descriptors,
                &batch_info,
            );

            scene2d_resources.draw_scene(
                &self.device,
                command_buffer,
                self.pipeline_objects.main_2d_graphics_pipeline,
                self.pipeline_layout_2d,
                &self.textures,
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
        )
        .unwrap();

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
        )
        .unwrap();

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

        let global_2d_bindings = [
            vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT),
            vk::DescriptorSetLayoutBinding::default()
                .binding(1)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT),
        ];

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
        glyph_atlas: &GlyphAtlas,
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
                    glyph_atlas,
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
