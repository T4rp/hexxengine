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

impl ImageTransition {
    pub fn as_barrier(&self) -> vk::ImageMemoryBarrier2<'_> {
        vk::ImageMemoryBarrier2::default()
            .src_stage_mask(self.src_stage)
            .src_access_mask(self.src_access)
            .dst_stage_mask(self.dst_stage)
            .dst_access_mask(self.dst_access)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .old_layout(self.current_layout)
            .new_layout(self.new_layout)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: self.aspect_mask,
                base_mip_level: 0,
                level_count: vk::REMAINING_MIP_LEVELS,
                base_array_layer: 0,
                layer_count: vk::REMAINING_ARRAY_LAYERS,
            })
            .image(self.image)
    }
}

pub fn transition_images(
    device: &ash::Device,
    command_buffer: vk::CommandBuffer,
    barriers: &[vk::ImageMemoryBarrier2],
) {
    let dep_info = vk::DependencyInfo::default().image_memory_barriers(barriers);

    unsafe { device.cmd_pipeline_barrier2(command_buffer, &dep_info) };
}
