use ash::vk;

pub struct ImageTransition {
    pub image: vk::Image,
    pub current_layout: vk::ImageLayout,
    pub new_layout: vk::ImageLayout,
    pub src_stage: vk::PipelineStageFlags2,
    pub src_access: vk::AccessFlags2,
    pub dst_stage: vk::PipelineStageFlags2,
    pub dst_access: vk::AccessFlags2,
    pub aspect_mask: vk::ImageAspectFlags,
}

pub fn transition_images(
    device: &ash::Device,
    command_buffer: vk::CommandBuffer,
    transitions: &[ImageTransition],
) {
    let mut image_barriers = Vec::with_capacity(transitions.len());

    for image_transition in transitions {
        let image_barrier = vk::ImageMemoryBarrier2::default()
            .src_stage_mask(image_transition.src_stage)
            .src_access_mask(image_transition.src_access)
            .dst_stage_mask(image_transition.dst_stage)
            .dst_access_mask(image_transition.dst_access)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .old_layout(image_transition.current_layout)
            .new_layout(image_transition.new_layout)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: image_transition.aspect_mask,
                base_mip_level: 0,
                level_count: vk::REMAINING_MIP_LEVELS,
                base_array_layer: 0,
                layer_count: vk::REMAINING_ARRAY_LAYERS,
            })
            .image(image_transition.image);

        image_barriers.push(image_barrier)
    }

    let dep_info = vk::DependencyInfo::default().image_memory_barriers(&image_barriers);

    unsafe { device.cmd_pipeline_barrier2(command_buffer, &dep_info) };
}
