use std::collections::VecDeque;

use ash::vk::{self};

use crate::renderer::renderer::MAX_FRAMES;

pub enum Resource {
    Swapchain(vk::SwapchainKHR),
    Image(vk::Image),
    AllocatedImage((vk::Image, vk_mem::Allocation)),
    ImageView(vk::ImageView),
    Semaphore(vk::Semaphore),
}

struct QueuedResource {
    future_frame: usize,
    resource: Resource,
}

pub struct VulkanDeletionQueue {
    pub current_frame: usize,
    queue: VecDeque<QueuedResource>,
}

impl VulkanDeletionQueue {
    pub fn new() -> VulkanDeletionQueue {
        let queue = VecDeque::new();
        Self {
            queue,
            current_frame: 0,
        }
    }

    pub fn push(&mut self, resource: Resource) {
        self.queue.push_back(QueuedResource {
            future_frame: self.current_frame + MAX_FRAMES,
            resource,
        })
    }

    pub fn clean(
        &mut self,
        allocator: &vk_mem::Allocator,
        device: &ash::Device,
        swapchain_fn: &ash::khr::swapchain::Device,
    ) {
        loop {
            let Some(resource) = self
                .queue
                .pop_back_if(|r| r.future_frame <= self.current_frame)
            else {
                break;
            };

            match resource.resource {
                Resource::Swapchain(swapchain) => unsafe {
                    swapchain_fn.destroy_swapchain(swapchain, None)
                },
                Resource::Image(image) => unsafe {
                    device.destroy_image(image, None);
                },
                Resource::AllocatedImage((image, mut allocation)) => unsafe {
                    allocator.destroy_image(image, &mut allocation);
                },
                Resource::ImageView(view) => unsafe {
                    device.destroy_image_view(view, None);
                },
                Resource::Semaphore(semaphore) => unsafe {
                    device.destroy_semaphore(semaphore, None);
                },
            }
        }
    }
}
