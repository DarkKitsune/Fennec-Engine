use super::framebuffer::Framebuffer;
use super::image::Image;
use super::queue::QueueFamilyCollection;
use super::renderpass::{RenderPass, Subpass};
use super::swapchain::Swapchain;
use super::sync::{Fence, Semaphore};
use super::vkobject::VKObject;
use super::Context;
use crate::error::FennecError;
use crate::iteratorext::IteratorResults;
use ash::vk;
use std::cell::RefCell;
use std::rc::Rc;

pub struct RenderTest {
    pub render_pass: RenderPass,
    pub framebuffers: Vec<Framebuffer>,
    pub finished_semaphore: Semaphore,
}

impl RenderTest {
    const COMMAND_BUFFERS_NAME: &'static str = "render_test";

    pub fn new(
        context: &Rc<RefCell<Context>>,
        queue_family_collection: &mut QueueFamilyCollection,
        swapchain: &Swapchain,
    ) -> Result<Self, FennecError> {
        // Create render finished semaphore
        let mut finished_semaphore = Semaphore::new(context)?;
        finished_semaphore.set_name("RenderTest finished semaphore")?;
        // Create render pass and framebuffers
        let attachments = [vk::AttachmentDescription::builder()
            .format(swapchain.format())
            .initial_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .final_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .samples(vk::SampleCountFlags::TYPE_1)
            .build()];
        let subpasses = [Subpass {
            input_attachments: vec![],
            color_attachments: vec![vk::AttachmentReference::builder()
                .attachment(0)
                .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .build()],
            depth_stencil_attachment: None,
            preserve_attachments: vec![],
            dependencies: vec![],
        }];
        let mut render_pass = RenderPass::new(context, &attachments, &subpasses)?;
        render_pass.set_name("RenderTest render pass")?;
        let framebuffers = swapchain
            .images()
            .iter()
            .enumerate()
            .map(|(index, image)| {
                let mut view = image.view(&image.range_color_basic(), None)?;
                view.set_name(&format!("RenderTest framebuffer {} image view", index))?;
                let mut framebuffer = Framebuffer::new(context, &render_pass, vec![view])?;
                framebuffer.set_name(&format!("RenderTest framebuffer {}", index))?;
                Ok(framebuffer)
            })
            .handle_results()?
            .collect::<Vec<Framebuffer>>();
        // Create command buffers
        let graphics_long_term = queue_family_collection
            .graphics_mut()
            .command_pools_mut()
            .unwrap()
            .long_term_mut();
        let mut buffers = graphics_long_term
            .create_command_buffers(Self::COMMAND_BUFFERS_NAME, swapchain.images().len() as u32)?;
        for (i, buffer) in buffers.iter_mut().enumerate() {
            let image = &swapchain.images()[i];
            let writer = buffer.begin(false, true)?;
            writer.pipeline_barrier(
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                None,
                None,
                None,
                Some(&[vk::ImageMemoryBarrier::builder()
                    .image(*image.image_handle().handle())
                    .old_layout(vk::ImageLayout::UNDEFINED)
                    .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .src_access_mask(Default::default())
                    .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                    .subresource_range(image.range_color_basic())
                    .build()]),
            )?;
            let pass = writer.begin_render_pass(
                &render_pass,
                &framebuffers[i],
                vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: swapchain.extent(),
                },
                &[vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.5, 0.7, 0.9, 1.0],
                    },
                }],
            )?;
            pass.end();
            writer.pipeline_barrier(
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                None,
                None,
                None,
                Some(&[vk::ImageMemoryBarrier::builder()
                    .image(*image.image_handle().handle())
                    .old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                    .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                    .dst_access_mask(vk::AccessFlags::MEMORY_READ)
                    .subresource_range(image.range_color_basic())
                    .build()]),
            )?;
        }
        Ok(Self {
            finished_semaphore,
            render_pass,
            framebuffers,
        })
    }

    pub fn submit(
        &self,
        wait_for: (&Semaphore, vk::PipelineStageFlags),
        queue_family_collection: &QueueFamilyCollection,
        image_index: u32,
        signaled_fence: Option<&Fence>,
    ) -> Result<&Semaphore, FennecError> {
        let graphics_family = queue_family_collection.graphics();
        let graphics_long_term = graphics_family.command_pools().unwrap().long_term();
        graphics_family.queue_of_priority(1.0).unwrap().submit(
            Some(&[
                graphics_long_term.command_buffers(Self::COMMAND_BUFFERS_NAME)?
                    [image_index as usize],
            ]),
            Some(&[wait_for]),
            Some(&[&self.finished_semaphore]),
            signaled_fence,
        )?;
        Ok(&self.finished_semaphore)
    }
}
