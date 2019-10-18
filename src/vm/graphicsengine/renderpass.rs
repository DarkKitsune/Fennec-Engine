use super::vkobject::{VKHandle, VKObject};
use super::Context;
use crate::error::FennecError;
use ash::version::DeviceV1_0;
use ash::vk;
use std::cell::RefCell;
use std::rc::Rc;

/// A render pass
pub struct RenderPass {
    render_pass: VKHandle<vk::RenderPass>,
}

impl RenderPass {
    /// RenderPass factory method
    pub fn new(
        context: &Rc<RefCell<Context>>,
        attachments: &[vk::AttachmentDescription],
        subpasses: &[Subpass],
    ) -> Result<Self, FennecError> {
        // Set render pass create info
        let subpass_infos = subpasses
            .iter()
            .enumerate()
            .map(|(index, _subpass)| {
                let builder = vk::SubpassDescription::builder()
                    .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
                    .input_attachments(&subpasses[index].input_attachments)
                    .color_attachments(&subpasses[index].color_attachments)
                    .preserve_attachments(&subpasses[index].preserve_attachments);
                if let Some(depth_stencil_attachment) = &subpasses[index].depth_stencil_attachment {
                    *builder.depth_stencil_attachment(&depth_stencil_attachment)
                } else {
                    *builder
                }
            })
            .collect::<Vec<vk::SubpassDescription>>();
        let dependencies = subpasses
            .iter()
            .enumerate()
            .map(|(index, subpass)| {
                subpass.dependencies.iter().map(move |&dependency| {
                    *vk::SubpassDependency::builder()
                        .src_subpass(match dependency.depends_on {
                            DependsOn::ExternalSubpass => vk::SUBPASS_EXTERNAL,
                            DependsOn::Subpass(depended_subpass) => depended_subpass,
                        })
                        .dst_subpass(index as u32)
                        .src_stage_mask(dependency.src_stage)
                        .dst_stage_mask(dependency.dst_stage)
                        .src_access_mask(dependency.src_access)
                        .dst_access_mask(dependency.dst_access)
                })
            })
            .flatten()
            .collect::<Vec<vk::SubpassDependency>>();
        let create_info = vk::RenderPassCreateInfo::builder()
            .attachments(attachments)
            .subpasses(&subpass_infos)
            .dependencies(&dependencies);
        // Create render pass
        let render_pass = unsafe {
            context
                .try_borrow()?
                .logical_device()
                .create_render_pass(&create_info, None)
        }?;
        // Return render pass
        Ok(Self {
            render_pass: VKHandle::new(context, render_pass, false),
        })
    }
}

impl VKObject<vk::RenderPass> for RenderPass {
    fn wrapped_handle(&self) -> &VKHandle<vk::RenderPass> {
        &self.render_pass
    }

    fn wrapped_handle_mut(&mut self) -> &mut VKHandle<vk::RenderPass> {
        &mut self.render_pass
    }

    fn object_type() -> vk::DebugReportObjectTypeEXT {
        vk::DebugReportObjectTypeEXT::RENDER_PASS
    }

    fn set_children_names(&mut self) -> Result<(), FennecError> {
        Ok(())
    }
}

/// Describes a single subpass in a render pass
#[derive(Default, Clone)]
pub struct Subpass {
    /// Input attachments for shaders
    pub input_attachments: Vec<vk::AttachmentReference>,
    /// Color attachments
    pub color_attachments: Vec<vk::AttachmentReference>,
    /// Depth/stencil attachment
    pub depth_stencil_attachment: Option<vk::AttachmentReference>,
    /// Indices of render pass attachments that aren't used but must be preserved through the subpass
    pub preserve_attachments: Vec<u32>,
    /// Subpasses in the render pass that the subpass depends on
    pub dependencies: Vec<Dependency>,
}

/// Describes a subpass' dependency on part of another subpass
#[derive(Default, Copy, Clone)]
pub struct Dependency {
    /// The depended-on subpass
    pub depends_on: DependsOn,
    /// Pipeline stage that must be finished with first by depended-on subpass
    pub src_stage: vk::PipelineStageFlags,
    /// Pipeline stage in this subpass that depends on this dependency
    pub dst_stage: vk::PipelineStageFlags,
    /// Type of access that must be finished with first by depended-on subpass
    pub src_access: vk::AccessFlags,
    /// Type of access in this subpass that depends on this dependency
    pub dst_access: vk::AccessFlags,
}

/// A subpass a dependency depends on
#[derive(Copy, Clone)]
pub enum DependsOn {
    ExternalSubpass,
    Subpass(u32),
}

impl Default for DependsOn {
    fn default() -> Self {
        Self::ExternalSubpass
    }
}
