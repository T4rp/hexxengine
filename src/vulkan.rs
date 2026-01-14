use std::borrow::Cow;
use std::io::Cursor;
use std::rc::Rc;
use std::time::SystemTime;
use std::{ffi, fs, mem};

use ash::Entry;
use ash::vk::ApplicationInfo;
use ash::vk::{
    self, DebugUtilsMessageSeverityFlagsEXT, DebugUtilsMessageTypeFlagsEXT,
    DebugUtilsMessengerCallbackDataEXT, DebugUtilsMessengerCreateInfoEXT,
};
use glam::{Quat, Vec3, vec2, vec3};
use vk_mem::Alloc;
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle};
use winit::window::Window;

use crate::mesh::{CameraUniform, Vertex2d};

const USE_VALIDATION_LAYERS: bool = true;
const MAX_FRAMES: usize = 2;

const VERTICES: &[Vertex2d] = &[
    Vertex2d {
        pos: vec2(-1.0, 1.0),
        uv: vec2(-1.0, 1.0),
        color: vec3(0.0, 1.0, 0.0),
    },
    Vertex2d {
        pos: vec2(1.0, 1.0),
        uv: vec2(-1.0, 1.0),
        color: vec3(1.0, 0.0, 0.0),
    },
    Vertex2d {
        pos: vec2(0.0, -1.0),
        uv: vec2(-1.0, 1.0),
        color: vec3(0.0, 0.0, 1.0),
    },
];

const DESCRIPTOR_RATIOS: &[(vk::DescriptorType, u32)] = &[
    (vk::DescriptorType::COMBINED_IMAGE_SAMPLER, 1),
    (vk::DescriptorType::UNIFORM_BUFFER, 1),
];

unsafe extern "system" fn debug_messager_callback(
    message_severity: DebugUtilsMessageSeverityFlagsEXT,
    message_type: DebugUtilsMessageTypeFlagsEXT,
    callback_data: *const DebugUtilsMessengerCallbackDataEXT,
    _userdata: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    unsafe {
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

        println!(
            "{message_severity:?}:\n{message_type:?} [{message_id_name} ({message_id_number})] : {message}\n",
        );

        let bt = std::backtrace::Backtrace::capture();
        println!("Backtrace:\n{bt}");

        vk::FALSE
    }
}

pub struct MeshBuffers {
    vertex_buffer: vk::Buffer,
    index_buffer: vk::Buffer,
    index_count: u32,
}

type PerFrameDescriptorData = (vk::Buffer, vk_mem::Allocation);

pub struct RenderFrame {
    command_pool: vk::CommandPool,
    command_buffer: vk::CommandBuffer,
    swapchain_semaphore: vk::Semaphore,
    in_flight_fence: vk::Fence,
    per_frame_set: vk::DescriptorSet,
    per_material_set: vk::DescriptorSet,
    per_frame_descriptor_data: PerFrameDescriptorData,
    depth_image_view: vk::ImageView,
    depth_image: (vk::Image, vk_mem::Allocation),
}

pub struct VulkanContext {
    entry: Entry,
    instance: ash::Instance,
    surface: vk::SurfaceKHR,
    device: ash::Device,
    swapchain: vk::SwapchainKHR,
    swapchain_images: Vec<vk::Image>,
    swapchain_image_views: Vec<vk::ImageView>,
    graphics_queue: vk::Queue,
    graphics_queue_family_index: u32,
    render_frames: Vec<RenderFrame>,
    submit_semaphores: Vec<vk::Semaphore>,
    current_frame: usize,
    graphics_pipeline: vk::Pipeline,
    surface_format: vk::SurfaceFormatKHR,
    swapchain_extent: vk::Extent2D,
    allocator: vk_mem::Allocator,
    vertex_buffer: (vk::Buffer, vk_mem::Allocation),
    should_resize: bool,
    physical_device: vk::PhysicalDevice,
    window: Rc<Window>,
    descriptor_set_layouts: Vec<vk::DescriptorSetLayout>,
    descriptor_pool: vk::DescriptorPool,
    graphics_pipeline_layout: vk::PipelineLayout,

    camera: Camera,
    last_frame_time: SystemTime,
}

pub struct Camera {
    position: Vec3,
    orientation: Quat,
    fov: f32,
}
impl Camera {
    fn new(position: Vec3, orientation: Quat, fov: f32) -> Self {
        Self {
            position,
            orientation,
            fov,
        }
    }
}

fn create_instance(entry: &ash::Entry, raw_display_handle: RawDisplayHandle) -> ash::Instance {
    let mut extensions = vec![ash::ext::debug_utils::NAME.as_ptr()];
    let mut validation_layers = vec![];

    if USE_VALIDATION_LAYERS {
        validation_layers.push(c"VK_LAYER_KHRONOS_validation".as_ptr())
    }

    let surface_extensions = ash_window::enumerate_required_extensions(raw_display_handle).unwrap();

    extensions.extend_from_slice(surface_extensions);

    let appinfo = ApplicationInfo::default()
        .application_name(c"HexxEngine")
        .api_version(ash::vk::API_VERSION_1_3);

    let create_info = vk::InstanceCreateInfo::default()
        .application_info(&appinfo)
        .enabled_extension_names(&extensions)
        .enabled_layer_names(&validation_layers);

    let instance = unsafe { entry.create_instance(&create_info, None).unwrap() };
    let debug_utils_fn = ash::ext::debug_utils::Instance::new(&entry, &instance);

    let messager_create_info = DebugUtilsMessengerCreateInfoEXT::default()
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
    entry: &ash::Entry,
    instance: &ash::Instance,
    device: &ash::Device,
    physical_device: vk::PhysicalDevice,
    surface: vk::SurfaceKHR,
    surface_format: vk::SurfaceFormatKHR,
    window: &Window,
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
    let surface_fn = ash::khr::surface::Instance::new(entry, instance);
    let swapchain_fn = ash::khr::swapchain::Device::new(instance, device);

    let surface_capabilities = unsafe {
        surface_fn
            .get_physical_device_surface_capabilities(physical_device, surface)
            .unwrap()
    };

    let surface_max_image_extent = surface_capabilities.max_image_extent;

    let image_extent = if surface_max_image_extent.width != u32::MAX {
        surface_max_image_extent
    } else {
        let window_dimensions = window.inner_position().unwrap();
        vk::Extent2D {
            width: window_dimensions.x as u32,
            height: window_dimensions.y as u32,
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
                .components(vk::ComponentMapping {
                    r: vk::ComponentSwizzle::IDENTITY,
                    g: vk::ComponentSwizzle::IDENTITY,
                    b: vk::ComponentSwizzle::IDENTITY,
                    a: vk::ComponentSwizzle::IDENTITY,
                })
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

fn setup_per_frame_descriptor(
    device: &ash::Device,
    allocator: &vk_mem::Allocator,
    descriptor_set: vk::DescriptorSet,
) -> PerFrameDescriptorData {
    let camera_uniform_buffer_info = vk::BufferCreateInfo::default()
        .usage(vk::BufferUsageFlags::UNIFORM_BUFFER)
        .size(mem::size_of::<CameraUniform>() as u64);

    let camera_uniform_alloc_info = vk_mem::AllocationCreateInfo {
        flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE
            | vk_mem::AllocationCreateFlags::MAPPED,
        usage: vk_mem::MemoryUsage::AutoPreferHost,

        ..Default::default()
    };

    let camera_uniform_buffer = unsafe {
        allocator
            .create_buffer(&camera_uniform_buffer_info, &camera_uniform_alloc_info)
            .unwrap()
    };

    let buff_info = [vk::DescriptorBufferInfo::default()
        .offset(0)
        .range(mem::size_of::<CameraUniform>() as u64)
        .buffer(camera_uniform_buffer.0)];

    let descriptor_write = [vk::WriteDescriptorSet::default()
        .dst_set(descriptor_set)
        .dst_binding(0)
        .dst_array_element(0)
        .descriptor_count(1)
        .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
        .buffer_info(&buff_info)];

    unsafe { device.update_descriptor_sets(&descriptor_write, &[]) };

    camera_uniform_buffer
}

fn create_depth_resources(
    device: &ash::Device,
    allocator: &vk_mem::Allocator,
    window_extent: vk::Extent2D,
) -> ((vk::Image, vk_mem::Allocation), vk::ImageView) {
    let image_info = vk::ImageCreateInfo::default()
        .image_type(vk::ImageType::TYPE_2D)
        .extent(vk::Extent3D {
            width: window_extent.width,
            height: window_extent.height,
            depth: 1,
        })
        .mip_levels(1)
        .array_layers(1)
        .format(vk::Format::D32_SFLOAT)
        .tiling(vk::ImageTiling::OPTIMAL)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
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

    let image_view_info = vk::ImageViewCreateInfo::default()
        .image(depth_image.0)
        .view_type(vk::ImageViewType::TYPE_2D)
        .format(vk::Format::D32_SFLOAT)
        .components(
            vk::ComponentMapping::default()
                .r(vk::ComponentSwizzle::IDENTITY)
                .g(vk::ComponentSwizzle::IDENTITY)
                .b(vk::ComponentSwizzle::IDENTITY),
        )
        .subresource_range(
            vk::ImageSubresourceRange::default()
                .aspect_mask(vk::ImageAspectFlags::DEPTH)
                .base_mip_level(0)
                .level_count(1)
                .base_array_layer(0)
                .layer_count(1),
        );

    let depth_image_view = unsafe { device.create_image_view(&image_view_info, None).unwrap() };

    (depth_image, depth_image_view)
}

fn create_render_frames(
    device: &ash::Device,
    allocator: &vk_mem::Allocator,
    descriptor_pool: vk::DescriptorPool,
    descriptor_set_layout: &[vk::DescriptorSetLayout],
    window_extent: vk::Extent2D,
    queue_family_index: u32,
) -> Vec<RenderFrame> {
    let frames: Vec<RenderFrame> = (0..MAX_FRAMES)
        .map(|_i| {
            let command_pool_create_info = vk::CommandPoolCreateInfo::default()
                .queue_family_index(queue_family_index)
                .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);

            let command_pool = unsafe {
                device
                    .create_command_pool(&command_pool_create_info, None)
                    .unwrap()
            };

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

            let descriptor_set_alloc_info = vk::DescriptorSetAllocateInfo::default()
                .descriptor_pool(descriptor_pool)
                .set_layouts(descriptor_set_layout);

            let descriptor_sets = unsafe {
                device
                    .allocate_descriptor_sets(&descriptor_set_alloc_info)
                    .unwrap()
            };

            let per_frame_descriptor_data =
                setup_per_frame_descriptor(device, allocator, descriptor_sets[0]);

            let (depth_image, depth_image_view) =
                create_depth_resources(device, allocator, window_extent);

            RenderFrame {
                command_pool,
                command_buffer,
                swapchain_semaphore,
                in_flight_fence,
                per_frame_set: descriptor_sets[0],
                per_material_set: descriptor_sets[1],
                per_frame_descriptor_data,
                depth_image,
                depth_image_view,
            }
        })
        .collect();

    frames
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

fn create_shader_module(device: &ash::Device, data: &[u8]) -> vk::ShaderModule {
    let mut cursor = Cursor::new(data);
    let spv = ash::util::read_spv(&mut cursor).unwrap();

    let create_info = vk::ShaderModuleCreateInfo::default().code(&spv);

    unsafe { device.create_shader_module(&create_info, None).unwrap() }
}

fn create_vertex_buffer(allocator: &vk_mem::Allocator) -> (vk::Buffer, vk_mem::Allocation) {
    let buffer_info = vk::BufferCreateInfo::default()
        .size((mem::size_of::<Vertex2d>() * VERTICES.len()) as u64)
        .usage(vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST);

    let alloc_info = vk_mem::AllocationCreateInfo {
        usage: vk_mem::MemoryUsage::AutoPreferHost,
        flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE
            | vk_mem::AllocationCreateFlags::MAPPED,
        ..Default::default()
    };

    unsafe { allocator.create_buffer(&buffer_info, &alloc_info).unwrap() }
}

impl VulkanContext {
    pub fn new(window: Rc<Window>) -> Self {
        let raw_window_handle = window.window_handle().unwrap().as_raw();
        let raw_display_handle = window.display_handle().unwrap().as_raw();

        let entry = unsafe { Entry::load().unwrap() };
        let instance = create_instance(&entry, raw_display_handle);
        let surface_fn = ash::khr::surface::Instance::new(&entry, &instance);

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
                    surface_fn
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

        let allocator_create_info =
            vk_mem::AllocatorCreateInfo::new(&instance, &device, physical_device);

        let allocator = unsafe { vk_mem::Allocator::new(allocator_create_info).unwrap() };

        let graphics_queue = unsafe { device.get_device_queue(graphics_queue_family_index, 0) };

        let descriptor_pool = Self::create_descriptor_pool(&device, 3);
        let descriptor_set_layouts = Self::create_descriptor_layouts(&device);

        let all_surface_formats = unsafe {
            surface_fn
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

        let (swapchain, swapchain_images, swapchain_image_views, swapchain_extent) =
            create_swapchain(
                &entry,
                &instance,
                &device,
                physical_device,
                surface,
                surface_format,
                &window,
                None,
            )
            .unwrap();

        let render_frames = create_render_frames(
            &device,
            &allocator,
            descriptor_pool,
            &descriptor_set_layouts,
            swapchain_extent,
            graphics_queue_family_index,
        );

        let submit_semaphores = create_submit_semaphores(&device, swapchain_images.len());

        let (graphics_pipeline, graphics_pipeline_layout) =
            Self::create_graphics_pipeline(&device, surface_format, &descriptor_set_layouts);

        let vertex_buffer = create_vertex_buffer(&allocator);
        let alloc_info = allocator.get_allocation_info(&vertex_buffer.1);

        unsafe {
            std::ptr::copy_nonoverlapping(
                VERTICES.as_ptr(),
                alloc_info.mapped_data.cast(),
                VERTICES.len(),
            )
        };

        let current_frame: usize = 0;
        let should_recreate_swapchain = false;

        let camera = Camera::new(vec3(0.0, 0.0, -5.0), Quat::IDENTITY, 90.0);

        let last_frame_time = SystemTime::now();

        Self {
            window,
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
            should_resize: should_recreate_swapchain,
            render_frames,
            submit_semaphores,
            current_frame,
            graphics_pipeline,
            graphics_pipeline_layout,
            allocator,
            vertex_buffer,
            descriptor_set_layouts,
            descriptor_pool,
            camera,
            last_frame_time,
        }
    }

    fn transition_image(
        &mut self,
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
                aspect_mask: aspect_mask,
                base_mip_level: 0,
                level_count: vk::REMAINING_MIP_LEVELS,
                base_array_layer: 0,
                layer_count: vk::REMAINING_ARRAY_LAYERS,
            })
            .image(image)];

        let dep_info = vk::DependencyInfo::default().image_memory_barriers(image_barriers);

        unsafe { self.device.cmd_pipeline_barrier2(command_buffer, &dep_info) };
    }

    pub fn update_per_frame_descriptors(&mut self) {
        let current_frame = &self.render_frames[self.current_frame % MAX_FRAMES];
        let camera_buffer_allocation = current_frame.per_frame_descriptor_data.1;

        let alloc_info = self
            .allocator
            .get_allocation_info(&camera_buffer_allocation);

        let camera = &self.camera;
        let aspect_ratio = self.swapchain_extent.width as f32 / self.swapchain_extent.height as f32;

        let mut camera_ubo = CameraUniform::new(
            camera.position,
            camera.orientation,
            camera.fov,
            aspect_ratio,
        );

        unsafe { std::ptr::copy_nonoverlapping(&mut camera_ubo, alloc_info.mapped_data.cast(), 1) };
    }

    pub fn update(&mut self) {
        let dt = self.last_frame_time.elapsed().unwrap().as_secs_f32();
        self.last_frame_time = SystemTime::now();
        self.camera.orientation *=
            Quat::from_euler(glam::EulerRot::XYZ, 0.0, f32::to_radians(100.0) * dt, 0.0);
    }

    pub fn draw(&mut self) {
        if self.should_resize {
            self.handle_resize();
            return;
        }

        let swapchain_fn = ash::khr::swapchain::Device::new(&self.instance, &self.device);

        let current_frame = &mut self.render_frames[self.current_frame % MAX_FRAMES];
        let command_pool = current_frame.command_pool;
        let command_buffer = current_frame.command_buffer;
        let swapchain_semaphore = current_frame.swapchain_semaphore;
        let in_flight_fence = current_frame.in_flight_fence;
        let per_frame_descriptor_set = current_frame.per_frame_set;
        let depth_image = current_frame.depth_image.0;
        let depth_image_view = current_frame.depth_image_view;

        unsafe {
            self.device
                .wait_for_fences(&[in_flight_fence], true, 1000000000)
                .unwrap();

            let (image_index, should_recreate) = match swapchain_fn.acquire_next_image(
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

            self.update_per_frame_descriptors();

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

            self.transition_image(
                command_buffer,
                swapchain_image,
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                vk::ImageAspectFlags::COLOR,
            );

            self.transition_image(
                command_buffer,
                depth_image,
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                vk::ImageAspectFlags::DEPTH,
            );

            let rendering_attachment = &[vk::RenderingAttachmentInfo::default()
                .image_view(swapchain_image_view)
                .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .clear_value(vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.0, 0.0, 0.0, 1.0],
                    },
                })];

            let depth_attachment = vk::RenderingAttachmentInfo::default()
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

            let render_area = vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: self.swapchain_extent,
            };

            let rendering_info = vk::RenderingInfo::default()
                .color_attachments(rendering_attachment)
                .depth_attachment(&depth_attachment)
                .render_area(render_area)
                .layer_count(1);

            self.device
                .cmd_begin_rendering(command_buffer, &rendering_info);

            self.device
                .cmd_set_scissor(command_buffer, 0, &[render_area]);

            self.device.cmd_set_viewport(
                command_buffer,
                0,
                &[vk::Viewport {
                    x: 0.0,
                    y: 0.0,
                    width: render_area.extent.width as f32,
                    height: render_area.extent.height as f32,
                    min_depth: 0.0,
                    max_depth: 1.0,
                }],
            );

            self.device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.graphics_pipeline,
            );

            let descriptor_sets = [per_frame_descriptor_set];

            self.device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.graphics_pipeline_layout,
                0,
                &descriptor_sets,
                &[],
            );

            self.device
                .cmd_bind_vertex_buffers(command_buffer, 0, &[self.vertex_buffer.0], &[0]);

            self.device.cmd_draw(command_buffer, 3, 1, 0, 0);

            self.device.cmd_end_rendering(command_buffer);

            self.transition_image(
                command_buffer,
                swapchain_image,
                vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                vk::ImageLayout::PRESENT_SRC_KHR,
                vk::ImageAspectFlags::COLOR,
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

            let should_recreate =
                match swapchain_fn.queue_present(self.graphics_queue, &present_info) {
                    Ok(r) => r,
                    Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => true,
                    Err(err) => panic!("{}", err),
                };

            if should_recreate {
                self.should_resize = true;
            }
        }

        self.current_frame = self.current_frame + 1;
    }

    pub fn handle_resize(&mut self) {
        unsafe { self.device.device_wait_idle().unwrap() };

        let swapchain_fn = ash::khr::swapchain::Device::new(&self.instance, &self.device);

        let (swapchain, swapchain_images, swapchain_image_views, swapchain_extent) =
            match create_swapchain(
                &self.entry,
                &self.instance,
                &self.device,
                self.physical_device,
                self.surface,
                self.surface_format,
                &self.window,
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

        unsafe { swapchain_fn.destroy_swapchain(self.swapchain, None) };

        let semaphore_create_info =
            vk::SemaphoreCreateInfo::default().flags(vk::SemaphoreCreateFlags::empty());

        for frame in self.render_frames.iter_mut() {
            unsafe {
                self.device
                    .destroy_semaphore(frame.swapchain_semaphore, None);
            };

            let new_semaphore = unsafe {
                self.device
                    .create_semaphore(&semaphore_create_info, None)
                    .unwrap()
            };

            unsafe {
                self.allocator
                    .destroy_image(frame.depth_image.0, &mut frame.depth_image.1);

                self.device.destroy_image_view(frame.depth_image_view, None);
            };

            let (depth_image, depth_image_view) =
                create_depth_resources(&self.device, &self.allocator, swapchain_extent);

            frame.swapchain_semaphore = new_semaphore;
            frame.depth_image = depth_image;
            frame.depth_image_view = depth_image_view;
        }

        self.swapchain = swapchain;
        self.swapchain_images = swapchain_images;
        self.swapchain_image_views = swapchain_image_views;
        self.swapchain_extent = swapchain_extent;
        self.should_resize = false;
    }

    fn create_graphics_pipeline(
        device: &ash::Device,
        surface_format: vk::SurfaceFormatKHR,
        descriptor_set_layouts: &[vk::DescriptorSetLayout],
    ) -> (vk::Pipeline, vk::PipelineLayout) {
        let pipeline_layout_info =
            vk::PipelineLayoutCreateInfo::default().set_layouts(descriptor_set_layouts);

        let graphics_pipeline_layout = unsafe {
            device
                .create_pipeline_layout(&pipeline_layout_info, None)
                .unwrap()
        };

        let dynamic_states = &[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];

        let dynamic_state_info =
            vk::PipelineDynamicStateCreateInfo::default().dynamic_states(dynamic_states);

        let viewports = &[vk::Viewport::default()];
        let scissors = &[vk::Rect2D::default()];

        let viewport_state_info = vk::PipelineViewportStateCreateInfo::default()
            .viewports(viewports)
            .scissors(scissors);

        let vert_shader_code = fs::read("shaders/tri.vert.spv").unwrap();
        let frag_shader_code = fs::read("shaders/tri.frag.spv").unwrap();

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
        let vertex_binding_descriptions = Vertex2d::get_binding_descriptions();

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

        let color_blender_state_info = vk::PipelineColorBlendStateCreateInfo::default()
            .attachments(color_blend_attachment_states);

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
            .layout(graphics_pipeline_layout)
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

        (graphics_pipeline, graphics_pipeline_layout)
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

    fn create_descriptor_layouts(device: &ash::Device) -> Vec<vk::DescriptorSetLayout> {
        let per_frame_bindings = [vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)];

        let per_frame_layout_info =
            vk::DescriptorSetLayoutCreateInfo::default().bindings(&per_frame_bindings);

        let per_material_bindings = [vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)];

        let per_material_layout_info =
            vk::DescriptorSetLayoutCreateInfo::default().bindings(&per_material_bindings);

        let mut descriptor_layouts = Vec::new();

        let per_frame_layout = unsafe {
            device
                .create_descriptor_set_layout(&per_frame_layout_info, None)
                .unwrap()
        };

        let per_material_layout = unsafe {
            device
                .create_descriptor_set_layout(&per_material_layout_info, None)
                .unwrap()
        };

        descriptor_layouts.push(per_frame_layout);
        descriptor_layouts.push(per_material_layout);

        descriptor_layouts
    }
}

impl Drop for VulkanContext {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.device_wait_idle();

            self.allocator
                .destroy_buffer(self.vertex_buffer.0, &mut self.vertex_buffer.1);

            for render_frame in self.render_frames.iter_mut() {
                self.allocator.destroy_buffer(
                    render_frame.per_frame_descriptor_data.0,
                    &mut render_frame.per_frame_descriptor_data.1,
                );

                self.allocator
                    .destroy_image(render_frame.depth_image.0, &mut render_frame.depth_image.1);
            }
        };
    }
}
