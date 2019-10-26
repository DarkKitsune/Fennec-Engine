use super::image::Image;
use super::queuefamily::{CommandBuffer, QueueFamilyCollection};
use super::swapchain::Swapchain;
use super::sync::{Fence, Semaphore};
use super::vkobject::VKObject;
use crate::cache::Handle;
use crate::error::FennecError;
use ash::vk;

pub struct PresentTransitioner {
    command_buffer_handle: Handle<Vec<CommandBuffer>>,
    finished_semaphore: Semaphore,
}

impl PresentTransitioner {
    pub fn new(
        queue_family_collection: &mut QueueFamilyCollection,
        swapchain: &Swapchain,
        initial_state: (vk::PipelineStageFlags, vk::ImageLayout, vk::AccessFlags),
    ) -> Result<Self, FennecError> {
        let (command_buffer_handle, command_buffers) = queue_family_collection
            .graphics_mut()
            .command_pools_mut()
            .unwrap()
            .long_term_mut()
            .create_command_buffers(swapchain.images().len() as u32)?;
        for (image_index, image) in swapchain.images().iter().enumerate() {
            let writer = command_buffers[image_index].begin(false, true)?;
            writer.pipeline_barrier(
                initial_state.0,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                None,
                None,
                None,
                Some(&[*vk::ImageMemoryBarrier::builder()
                    .image(image.handle())
                    .subresource_range(image.range_color_basic())
                    .old_layout(initial_state.1)
                    .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                    .src_access_mask(initial_state.2)
                    .dst_access_mask(vk::AccessFlags::MEMORY_READ)]),
            )?;
        }
        let finished_semaphore = Semaphore::new(swapchain.context())?;
        Ok(Self {
            command_buffer_handle,
            finished_semaphore,
        })
    }

    pub fn submit(
        &self,
        wait_for: &Semaphore,
        queue_family_collection: &QueueFamilyCollection,
        image_index: u32,
        signaled_fence: Option<&Fence>,
    ) -> Result<&Semaphore, FennecError> {
        let command_buffers = queue_family_collection
            .graphics()
            .command_pools()
            .unwrap()
            .long_term()
            .command_buffers(self.command_buffer_handle)?;
        queue_family_collection
            .graphics()
            .queue_of_priority(1.0)
            .unwrap()
            .submit(
                Some(&[&command_buffers[image_index as usize]]),
                Some(&[(&wait_for, vk::PipelineStageFlags::BOTTOM_OF_PIPE)]),
                Some(&[&self.finished_semaphore]),
                signaled_fence,
            )?;
        Ok(&self.finished_semaphore)
    }
}
