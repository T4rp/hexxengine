use std::borrow::Cow;
use std::ffi;
use std::rc::Rc;

use ash::Entry;
use ash::vk::{
    self, ApplicationInfo, DebugUtilsMessageSeverityFlagsEXT, DebugUtilsMessageTypeFlagsEXT,
    DebugUtilsMessengerCallbackDataEXT, DebugUtilsMessengerCreateInfoEXT,
};
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle};
use winit::window::Window;

const USE_VALIDATION_LAYERS: bool = true;
const MAX_FRAMES: usize = 2;

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

        vk::FALSE
    }
}

pub struct RenderFrame {
    command_pool: vk::CommandPool,
    command_buffer: vk::CommandBuffer,
    swapchain_semaphore: vk::Semaphore,
    render_semaphore: vk::Semaphore,
    in_flight_fence: vk::Fence,
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
    current_frame: usize,
}

impl VulkanContext {
    pub fn new(window: Rc<Window>) -> Self {
        let raw_window_handle = window.window_handle().unwrap().as_raw();
        let raw_display_handle = window.display_handle().unwrap().as_raw();

        let entry = unsafe { Entry::load().unwrap() };
        let instance = Self::create_instance(&entry, raw_display_handle);
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
                if props.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
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

        let device_create_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(device_queue_create_infos)
            .enabled_extension_names(device_extensions);

        let device = unsafe {
            instance
                .create_device(physical_device, &device_create_info, None)
                .unwrap()
        };

        let graphics_queue = unsafe { device.get_device_queue(graphics_queue_family_index, 0) };

        let render_frames = Self::create_render_frames(&device, graphics_queue_family_index);

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

        let (swapchain, swapchain_images, swapchain_image_views) = Self::create_swapchain(
            &entry,
            &instance,
            &device,
            physical_device,
            surface,
            surface_format,
            &window,
        );

        let current_frame: usize = 0;

        Self {
            entry,
            instance,
            surface,
            device,
            graphics_queue_family_index,
            graphics_queue,
            swapchain,
            swapchain_images,
            swapchain_image_views,
            render_frames,
            current_frame,
        }
    }

    pub fn draw(&mut self) {
        let current_frame = &self.render_frames[self.current_frame];

        unsafe {
            self.device
                .wait_for_fences(&[current_frame.in_flight_fence], true, u64::MAX)
                .unwrap();

            self.device
                .reset_fences(&[current_frame.in_flight_fence])
                .unwrap();
        }
    }

    fn create_instance(entry: &ash::Entry, raw_display_handle: RawDisplayHandle) -> ash::Instance {
        let mut extensions = vec![ash::ext::debug_utils::NAME.as_ptr()];
        let mut validation_layers = vec![];

        if USE_VALIDATION_LAYERS {
            validation_layers.push(c"VK_LAYER_KHRONOS_validation".as_ptr())
        }

        let surface_extensions =
            ash_window::enumerate_required_extensions(raw_display_handle).unwrap();

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
        entry: &ash::Entry,
        instance: &ash::Instance,
        device: &ash::Device,
        physical_device: vk::PhysicalDevice,
        surface: vk::SurfaceKHR,
        surface_format: vk::SurfaceFormatKHR,
        window: &Window,
    ) -> (vk::SwapchainKHR, Vec<vk::Image>, Vec<vk::ImageView>) {
        let surface_fn = ash::khr::surface::Instance::new(entry, instance);
        let swapchain_fn = ash::khr::swapchain::Device::new(instance, device);

        let surface_capabilities = unsafe {
            surface_fn
                .get_physical_device_surface_capabilities(physical_device, surface)
                .unwrap()
        };

        let surface_min_image_extent = surface_capabilities.min_image_extent;
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
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_extent(image_extent)
            .old_swapchain(vk::SwapchainKHR::null());

        let swapchain = unsafe {
            swapchain_fn
                .create_swapchain(&create_swapchain_info, None)
                .unwrap()
        };

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
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    });

                unsafe { device.create_image_view(&image_create_info, None).unwrap() }
            })
            .collect();

        (swapchain, swapchain_images, image_views)
    }

    fn create_render_frames(device: &ash::Device, queue_family_index: u32) -> Vec<RenderFrame> {
        let frames: Vec<RenderFrame> = (0..MAX_FRAMES)
            .into_iter()
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

                let semaphore_create_info = vk::SemaphoreCreateInfo::default();

                let fence_create_info =
                    vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);

                let swapchain_semaphore = unsafe {
                    device
                        .create_semaphore(&semaphore_create_info, None)
                        .unwrap()
                };

                let render_semaphore = unsafe {
                    device
                        .create_semaphore(&semaphore_create_info, None)
                        .unwrap()
                };

                let in_flight_fence =
                    unsafe { device.create_fence(&fence_create_info, None).unwrap() };

                RenderFrame {
                    command_pool,
                    command_buffer,
                    swapchain_semaphore,
                    render_semaphore,
                    in_flight_fence,
                }
            })
            .collect();

        frames
    }
}
