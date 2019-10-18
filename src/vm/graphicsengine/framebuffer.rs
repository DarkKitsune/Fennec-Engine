use super::imageview::ImageView;
use super::renderpass::RenderPass;
use super::vkobject::{VKHandle, VKObject};
use super::Context;
use crate::error::FennecError;
use ash::version::DeviceV1_0;
use ash::vk;
use std::cell::RefCell;
use std::rc::Rc;

/// A framebuffer
pub struct Framebuffer {
    framebuffer: VKHandle<vk::Framebuffer>,
    attachments: Vec<ImageView>,
}

impl Framebuffer {
    /// Framebuffer factory method
    pub fn new(
        context: &Rc<RefCell<Context>>,
        render_pass: &RenderPass,
        attachments: Vec<ImageView>,
    ) -> Result<Self, FennecError> {
        let attachment_handles = attachments
            .iter()
            .map(|view| view.handle())
            .collect::<Vec<vk::ImageView>>();
        // Set framebuffer create info
        let create_info = vk::FramebufferCreateInfo::builder()
            .render_pass(render_pass.handle())
            .attachments(&attachment_handles)
            .width(
                attachments
                    .iter()
                    .map(|view| view.extent().width)
                    .max()
                    .unwrap_or(1),
            )
            .height(
                attachments
                    .iter()
                    .map(|view| view.extent().height)
                    .max()
                    .unwrap_or(1),
            )
            .layers(
                attachments
                    .iter()
                    .map(|view| view.extent().depth)
                    .max()
                    .unwrap_or(1),
            );
        // Create framebuffer
        let framebuffer = unsafe {
            context
                .try_borrow()?
                .logical_device()
                .create_framebuffer(&create_info, None)
        }?;
        // Return framebuffer
        Ok(Self {
            framebuffer: VKHandle::new(context, framebuffer, false),
            attachments,
        })
    }

    pub fn attachments(&self) -> &Vec<ImageView> {
        &self.attachments
    }
}

impl VKObject<vk::Framebuffer> for Framebuffer {
    fn wrapped_handle(&self) -> &VKHandle<vk::Framebuffer> {
        &self.framebuffer
    }

    fn wrapped_handle_mut(&mut self) -> &mut VKHandle<vk::Framebuffer> {
        &mut self.framebuffer
    }

    fn object_type() -> vk::DebugReportObjectTypeEXT {
        vk::DebugReportObjectTypeEXT::FRAMEBUFFER
    }

    fn set_children_names(&mut self) -> Result<(), FennecError> {
        Ok(())
    }
}
