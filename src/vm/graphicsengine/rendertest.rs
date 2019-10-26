use super::buffer::Buffer;
use super::descriptorpool::{Descriptor, DescriptorPool, DescriptorSet, DescriptorSetLayout};
use super::framebuffer::Framebuffer;
use super::image::{Image, Image2D};
use super::imageview::ImageView;
use super::pipeline::{BlendState, GraphicsPipeline, GraphicsStates, Viewport};
use super::queuefamily::CommandBuffer;
use super::queuefamily::QueueFamilyCollection;
use super::renderpass::{RenderPass, Subpass};
use super::sampler::{Filters, Sampler};
use super::shadermodule::ShaderModule;
use super::swapchain::Swapchain;
use super::sync::{Fence, Semaphore};
use super::vkobject::VKObject;
use super::Context;
use crate::cache::Handle;
use crate::error::FennecError;
use crate::iteratorext::IteratorResults;
use crate::vm::contentengine::{ContentEngine, ContentType};
use ash::vk;
use image::{GenericImageView, ImageFormat};
use std::cell::RefCell;
use std::ffi::CString;
use std::io::BufReader;
use std::ops::Deref;
use std::rc::Rc;

pub struct RenderTest {
    _pipeline: RenderTestPipeline,
    finished_semaphore: Semaphore,
    command_buffers_handle: Handle<Vec<CommandBuffer>>,
    _color_uniform_buffer: Buffer,
    _texture_image: Image2D,
    _texture_image_view: ImageView,
    _texture_sampler: Sampler,
}

impl RenderTest {
    /// Factory method
    pub fn new(
        swapchain: &Swapchain,
        queue_family_collection: &mut QueueFamilyCollection,
    ) -> Result<Self, FennecError> {
        // Create pipeline
        let pipeline = RenderTestPipeline::new(swapchain.context(), swapchain)?;
        // Create render finished semaphore
        let finished_semaphore =
            Semaphore::new(swapchain.context())?.with_name("RenderTest::finished_semaphore")?;
        // Create color uniform buffer
        let mut color_uniform_buffer = Buffer::new(
            swapchain.context(),
            std::mem::size_of::<(f32, f32, f32, f32)>() as u64 * 3,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            None,
            None,
        )?
        .with_name("RenderTest::color_uniform_buffer")?;
        {
            let mapped = color_uniform_buffer.memory_mut().map_all()?;
            unsafe {
                let ptr = mapped.ptr() as *mut (f32, f32, f32, f32);
                *ptr = (1.0, 0.0, 0.0, 1.0);
                *ptr.offset(1) = (0.0, 1.0, 0.0, 1.0);
                *ptr.offset(2) = (0.0, 0.0, 1.0, 1.0);
            }
        }
        // Create texture
        let texture_source = image::load(
            BufReader::new(ContentEngine::open("test", ContentType::Image)?),
            ImageFormat::PNG,
        )?;
        let texture_image = Image2D::new(
            swapchain.context(),
            vk::Extent2D {
                width: texture_source.width(),
                height: texture_source.height(),
            },
            vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
            &[queue_family_collection.graphics()],
            Some(vk::Format::B8G8R8A8_UNORM),
            Some(vk::ImageLayout::UNDEFINED),
            None,
        )?
        .with_name("RenderTest::texture_image")?;
        texture_image.load_compressed_image(
            queue_family_collection,
            &texture_source,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            vk::AccessFlags::SHADER_READ,
        )?;
        let texture_image_view = texture_image
            .view(&texture_image.range_color_basic(), None)?
            .with_name("RenderTest::texture_image_view")?;
        // Create sampler
        let texture_sampler = Sampler::new(
            swapchain.context(),
            Filters {
                min: vk::Filter::NEAREST,
                mag: vk::Filter::NEAREST,
            },
            Default::default(),
            Default::default(),
            &Default::default(),
        )?
        .with_name("RenderTest::texture_sampler")?;
        // Update descriptor set
        let descriptor_set = pipeline.descriptor_set()?;
        pipeline.descriptor_pool.update_descriptor_sets(&[
            *vk::WriteDescriptorSet::builder()
                .dst_set(descriptor_set.handle())
                .dst_binding(0)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(&[*vk::DescriptorBufferInfo::builder()
                    .buffer(color_uniform_buffer.handle())
                    .offset(0)
                    .range(color_uniform_buffer.size())]),
            *vk::WriteDescriptorSet::builder()
                .dst_set(descriptor_set.handle())
                .dst_binding(1)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(&[*vk::DescriptorImageInfo::builder()
                    .image_view(texture_image_view.handle())
                    .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                    .sampler(texture_sampler.handle())]),
        ])?;
        // Create command buffers
        let (command_buffers_handle, command_buffers) = queue_family_collection
            .graphics_mut()
            .command_pools_mut()
            .unwrap()
            .long_term_mut()
            .create_command_buffers(swapchain.images().len() as u32)?;
        for (i, command_buffer) in command_buffers.iter_mut().enumerate() {
            let image = &swapchain.images()[i];
            let writer = command_buffer.begin(false, true)?;
            // Pipeline barrier for swapchain image
            // We need to transition it to be optimal for color attachment output
            writer.pipeline_barrier(
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                None,
                None,
                None,
                Some(&[*vk::ImageMemoryBarrier::builder()
                    .image(image.image_handle().handle())
                    .old_layout(vk::ImageLayout::UNDEFINED)
                    .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .src_access_mask(Default::default())
                    .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                    .subresource_range(image.range_color_basic())]),
            )?;
            {
                // Begin render pass
                let active_pass = writer.begin_render_pass(
                    &pipeline.render_pass,
                    &pipeline.framebuffers[i],
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
                    let active_pipeline = active_pass.bind_graphics_pipeline(&pipeline.pipeline)?;
                    // Bind descriptor set
                    active_pipeline.bind_descriptor_sets(&[pipeline.descriptor_set()?], 0)?;
                    // Draw
                    active_pipeline.draw(0, 3, 0, 1)?;
                }
            }
        }
        // Return new RenderTest
        Ok(Self {
            _pipeline: pipeline,
            finished_semaphore,
            command_buffers_handle,
            _color_uniform_buffer: color_uniform_buffer,
            _texture_image: texture_image,
            _texture_image_view: texture_image_view,
            _texture_sampler: texture_sampler,
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
                &graphics_long_term.command_buffers(self.command_buffers_handle)?
                    [image_index as usize],
            ]),
            Some(&[(wait_for, vk::PipelineStageFlags::TOP_OF_PIPE)]),
            Some(&[&self.finished_semaphore]),
            signaled_fence,
        )?;
        Ok(&self.finished_semaphore)
    }
}

/// RenderTest's pipeline and associated objects
struct RenderTestPipeline {
    render_pass: RenderPass,
    framebuffers: Vec<Framebuffer>,
    descriptor_pool: DescriptorPool,
    _descriptor_set_layout: Rc<RefCell<DescriptorSetLayout>>,
    descriptor_set_handle: Handle<Vec<DescriptorSet>>,
    _vertex_shader: ShaderModule,
    _fragment_shader: ShaderModule,
    pipeline: GraphicsPipeline,
}

impl RenderTestPipeline {
    /// Factory method
    fn new(context: &Rc<RefCell<Context>>, swapchain: &Swapchain) -> Result<Self, FennecError> {
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
                .final_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL),
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
            .with_name("RenderTestPipeline::render_pass")?;
        // Create framebuffers
        let framebuffers = swapchain
            .images()
            .iter()
            .enumerate()
            .map(|(index, image)| {
                let view = image
                    .view(&image.range_color_basic(), None)?
                    .with_name(&format!(
                        "RenderTestPipeline::framebuffers[{}].attachments[0]",
                        index
                    ))?;
                let framebuffer = Framebuffer::new(context, &render_pass, vec![view])?
                    .with_name(&format!("RenderTestPipeline::framebuffers[{}]", index))?;
                Ok(framebuffer)
            })
            .handle_results()?
            .collect::<Vec<Framebuffer>>();
        // Create descriptor pool
        let descriptor_set_layout = DescriptorSetLayout::new(
            context,
            1,
            vec![
                Descriptor {
                    shader_stage: vk::ShaderStageFlags::VERTEX,
                    shader_binding_location: 0,
                    descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                    count: 1,
                },
                Descriptor {
                    shader_stage: vk::ShaderStageFlags::FRAGMENT,
                    shader_binding_location: 1,
                    descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    count: 1,
                },
            ],
        )?
        .with_name("RenderTestPipeline::descriptor_set_layout")?;
        let mut descriptor_pool = DescriptorPool::new(context, &[&descriptor_set_layout], None)?
            .with_name("RenderTestPipeline::descriptor_pool")?;
        let descriptor_set_layout = Rc::new(RefCell::new(descriptor_set_layout));
        let (descriptor_set_handle, _) =
            descriptor_pool.create_descriptor_sets(&descriptor_set_layout)?;
        // Create vertex shader
        let vertex_shader = ShaderModule::new(
            context,
            &mut ContentEngine::open("test.vert", ContentType::ShaderModule)?,
        )?
        .with_name("RenderTestPipeline::vertex_shader")?;
        let vertex_entry = CString::new(vertex_shader.entry_point())?;
        // Create fragment shader
        let fragment_shader = ShaderModule::new(
            context,
            &mut ContentEngine::open("test.frag", ContentType::ShaderModule)?,
        )?
        .with_name("RenderTestPipeline::fragment_shader")?;
        let fragment_entry = CString::new(fragment_shader.entry_point())?;
        // Create stages
        let stages = [
            *vk::PipelineShaderStageCreateInfo::builder()
                .module(vertex_shader.handle())
                .stage(vk::ShaderStageFlags::VERTEX)
                .name(&vertex_entry),
            *vk::PipelineShaderStageCreateInfo::builder()
                .module(fragment_shader.handle())
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
            &[descriptor_set_layout.try_borrow()?.deref()],
            &[],
            vk::PrimitiveTopology::TRIANGLE_LIST,
            &stages,
            &viewports,
            &graphics_states,
            None,
        )?
        .with_name("RenderTestPipeline::pipeline")?;
        Ok(Self {
            render_pass,
            framebuffers,
            descriptor_pool,
            _descriptor_set_layout: descriptor_set_layout,
            descriptor_set_handle,
            _vertex_shader: vertex_shader,
            _fragment_shader: fragment_shader,
            pipeline,
        })
    }

    /// Gets the descriptor set
    fn descriptor_set(&self) -> Result<&DescriptorSet, FennecError> {
        Ok(&self
            .descriptor_pool
            .descriptor_sets(self.descriptor_set_handle)?[0])
    }
}
