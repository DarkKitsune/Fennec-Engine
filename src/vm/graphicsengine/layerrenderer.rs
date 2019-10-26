use super::queuefamily::QueueFamilyCollection;
use super::sync::{Fence, Semaphore};
use crate::error::FennecError;
use ash::vk;

/// The trait uniting layer renderers
pub trait LayerRenderer {
    fn final_stage(&self) -> vk::PipelineStageFlags;
    fn final_layout(&self) -> vk::ImageLayout;
    fn final_access(&self) -> vk::AccessFlags;

    fn submit_draw(
        &self,
        wait_for: &Semaphore,
        queue_family_collection: &QueueFamilyCollection,
        image_index: u32,
        signaled_fence: Option<&Fence>,
    ) -> Result<&Semaphore, FennecError>;
}
