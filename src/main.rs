use std::borrow::Cow;
use std::ffi;
use std::rc::Rc;

use ash::Entry;
use ash::vk::{
    self, ApplicationInfo, DebugUtilsMessageSeverityFlagsEXT, DebugUtilsMessageTypeFlagsEXT,
    DebugUtilsMessengerCallbackDataEXT, DebugUtilsMessengerCreateInfoEXT,
};
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes},
};

const USE_VALIDATION_LAYERS: bool = true;

struct App {
    window: Option<Rc<Window>>,
    vk_ctx: Option<VulkanContext>,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            vk_ctx: None,
        }
    }
}

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

struct VulkanContext {
    entry: Entry,
    instance: ash::Instance,
    surface: vk::SurfaceKHR,
}

impl VulkanContext {
    fn new(window: Rc<Window>) -> Self {
        let raw_window_handle = window.window_handle().unwrap().as_raw();
        let raw_display_handle = window.display_handle().unwrap().as_raw();

        unsafe {
            let entry = Entry::load().unwrap();

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

            let instance = entry.create_instance(&create_info, None).unwrap();
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

            debug_utils_fn
                .create_debug_utils_messenger(&messager_create_info, None)
                .unwrap();

            let surface = ash_window::create_surface(
                &entry,
                &instance,
                raw_display_handle,
                raw_window_handle,
                None,
            )
            .unwrap();

            Self {
                entry,
                instance,
                surface,
            }
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = Rc::new(
            event_loop
                .create_window(WindowAttributes::default())
                .unwrap(),
        );

        let vk_ctx = VulkanContext::new(window.clone());

        self.window = Some(window);
        self.vk_ctx = Some(vk_ctx);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => {
                if PhysicalKey::Code(KeyCode::Escape) == event.physical_key {
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut app = App::new();
    event_loop.run_app(&mut app).unwrap();
}
