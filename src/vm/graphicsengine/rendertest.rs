use super::framebuffer::Framebuffer;
use super::image::Image;
use super::pipeline::{BlendState, GraphicsPipeline, GraphicsStates, Viewport};
use super::queue::QueueFamilyCollection;
use super::renderpass::{RenderPass, Subpass};
use super::shadermodule::ShaderModule;
use super::swapchain::Swapchain;
use super::sync::{Fence, Semaphore};
use super::vkobject::VKObject;
use super::Context;
use crate::error::FennecError;
use crate::iteratorext::IteratorResults;
use crate::paths;
use ash::vk;
use std::cell::RefCell;
use std::ffi::CString;
use std::fs::File;
use std::path::PathBuf;
use std::rc::Rc;

pub struct RenderTest {
    pub render_pass: RenderPass,
    pub framebuffers: Vec<Framebuffer>,
    pub finished_semaphore: Semaphore,
    pub vertex_shader: ShaderModule,
    pub fragment_shader: ShaderModule,
    pub pipeline: GraphicsPipeline,
}

impl RenderTest {
    const COMMAND_BUFFERS_NAME: &'static str = "render_test";

    /// Factory method
    pub fn new(
        context: &Rc<RefCell<Context>>,
        queue_family_collection: &mut QueueFamilyCollection,
        swapchain: &Swapchain,
    ) -> Result<Self, FennecError> {
        // Create render finished semaphore
        let finished_semaphore =
            Semaphore::new(context)?.with_name("RenderTest::finished_semaphore")?;
        // Create render pass
        let attachments = [
            // Color attachment
            *vk::AttachmentDescription::builder()
                .format(swapchain.format())
                .samples(vk::SampleCountFlags::TYPE_1)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                .initial_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .final_layout(vk::ImageLayout::PRESENT_SRC_KHR),
        ];
        let subpasses = [Subpass {
            input_attachments: vec![],
            color_attachments: vec![*vk::AttachmentReference::builder()
                .attachment(0)
                .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)],
            depth_stencil_attachment: None,
            preserve_attachments: vec![],
            dependencies: vec![],
        }];
        let render_pass = RenderPass::new(context, &attachments, &subpasses)?
            .with_name("RenderTest::render_pass")?;
        // Create framebuffers
        let framebuffers = swapchain
            .images()
            .iter()
            .enumerate()
            .map(|(index, image)| {
                let view = image
                    .view(&image.range_color_basic(), None)?
                    .with_name(&format!(
                        "RenderTest::framebuffers[{}].attachments[0]",
                        index
                    ))?;
                let framebuffer = Framebuffer::new(context, &render_pass, vec![view])?
                    .with_name(&format!("RenderTest::framebuffers[{}]", index))?;
                Ok(framebuffer)
            })
            .handle_results()?
            .collect::<Vec<Framebuffer>>();
        // Create vertex shader
        let vertex_shader = ShaderModule::new(
            context,
            &mut File::open(&paths::SHADERS.join(PathBuf::from("test.vert.spv")))?,
        )?
        .with_name("RenderTest::vertex_shader")?;
        let vertex_entry = CString::new(vertex_shader.entry_point())?;
        // Create fragment shader
        let fragment_shader = ShaderModule::new(
            context,
            &mut File::open(&paths::SHADERS.join(PathBuf::from("test.frag.spv")))?,
        )?
        .with_name("RenderTest::fragment_shader")?;
        let fragment_entry = CString::new(fragment_shader.entry_point())?;
        // Create stages
        let stages = [
            *vk::PipelineShaderStageCreateInfo::builder()
                .module(*vertex_shader.handle().handle())
                .stage(vk::ShaderStageFlags::VERTEX)
                .name(&vertex_entry),
            *vk::PipelineShaderStageCreateInfo::builder()
                .module(*fragment_shader.handle().handle())
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .name(&fragment_entry),
        ];
        // Create viewports
        let viewports = [Viewport {
            x: 0.0,
            y: 0.0,
            width: swapchain.extent().width as f32,
            height: swapchain.extent().height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
            scissor_offset: vk::Offset2D { x: 0, y: 0 },
            scissor_extent: swapchain.extent(),
        }];
        // Create graphics states
        let graphics_states = GraphicsStates {
            culling_state: Default::default(),
            depth_state: Default::default(),
            blend_state: BlendState {
                enable_logic_op: false,
                color_attachment_blend_functions: vec![
                    *vk::PipelineColorBlendAttachmentState::builder()
                        .blend_enable(true)
                        .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
                        .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_DST_ALPHA)
                        .color_blend_op(vk::BlendOp::ADD)
                        .src_alpha_blend_factor(vk::BlendFactor::ONE)
                        .dst_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_DST_ALPHA)
                        .alpha_blend_op(vk::BlendOp::ADD)
                        .color_write_mask(
                            vk::ColorComponentFlags::R
                                | vk::ColorComponentFlags::G
                                | vk::ColorComponentFlags::B
                                | vk::ColorComponentFlags::A,
                        ),
                ],
                ..Default::default()
            },
        };
        // Create pipeline
        let pipeline = GraphicsPipeline::new(
            context,
            &render_pass,
            0,
            &[],
            &[],
            vk::PrimitiveTopology::TRIANGLE_LIST,
            &stages,
            &viewports,
            &graphics_states,
            None,
        )?
        .with_name("RenderTest::pipeline")?;
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
                Some(&[*vk::ImageMemoryBarrier::builder()
                    .image(*image.image_handle().handle())
                    .old_layout(vk::ImageLayout::UNDEFINED)
                    .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .src_access_mask(Default::default())
                    .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                    .subresource_range(image.range_color_basic())]),
            )?;
            {
                // Begin render pass
                let active_pass = writer.begin_render_pass(
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
                {
                    // Begin pipeline
                    let active_pipeline = active_pass.bind_graphics_pipeline(&pipeline)?;
                    active_pipeline.draw(0, 3, 0, 1)?;
                }
            }
        }
        Ok(Self {
            render_pass,
            framebuffers,
            finished_semaphore,
            vertex_shader,
            fragment_shader,
            pipeline,
        })
    }

    /// Submit draw command buffers
    pub fn submit_draw(
        &self,
        wait_for: &Semaphore,
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
            Some(&[(wait_for, vk::PipelineStageFlags::TOP_OF_PIPE)]),
            Some(&[&self.finished_semaphore]),
            signaled_fence,
        )?;
        Ok(&self.finished_semaphore)
    }
}
