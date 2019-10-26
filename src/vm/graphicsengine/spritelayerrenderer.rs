use super::buffer::Buffer;
use super::descriptorpool::{Descriptor, DescriptorPool, DescriptorSet, DescriptorSetLayout};
use super::framebuffer::Framebuffer;
use super::image::{Image, Image2D};
use super::imageview::ImageView;
use super::layerrenderer::LayerRenderer;
use super::pipeline::{
    AttributeFormat, BlendState, GraphicsPipeline, GraphicsStates, VertexInputAttribute,
    VertexInputBinding, Viewport,
};
use super::queuefamily::{CommandBuffer, QueueFamilyCollection};
use super::renderpass::{RenderPass, Subpass};
use super::sampler::Sampler;
use super::shadermodule::ShaderModule;
use super::spritelayer::SpriteLayer;
use super::swapchain::Swapchain;
use super::sync::{Fence, Semaphore};
use super::tileregion::TileRegion;
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
use std::rc::Rc;

/// Renders the contents of a sprite layer
pub struct SpriteLayerRenderer {
    pipeline: SpritePipeline,
    _descriptor_set_handle: Handle<Vec<DescriptorSet>>,
    command_buffer_handle: Handle<Vec<CommandBuffer>>,
    _graphics_queue_family_index: u32,
    _texture_image: Image2D,
    _texture_view: ImageView,
    _instance_buffer: Buffer,
}

impl SpriteLayerRenderer {
    pub fn new(
        queue_family_collection: &mut QueueFamilyCollection,
        swapchain: &Swapchain,
        initial_state: Option<(vk::PipelineStageFlags, vk::ImageLayout, vk::AccessFlags)>,
    ) -> Result<Self, FennecError> {
        // Create pipeline
        let mut pipeline = SpritePipeline::new(swapchain.context(), swapchain)?;
        // Load texture image
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
            vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
            &[queue_family_collection.graphics()],
            Some(vk::Format::B8G8R8A8_UNORM),
            None,
            None,
        )?
        .with_name("SpriteLayerRenderer::texture_image")?;
        texture_image.load_compressed_image(
            queue_family_collection,
            &texture_source,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            vk::AccessFlags::SHADER_READ,
        )?;
        let texture_view = texture_image.view(&texture_image.range_color_basic(), None)?;
        // Create descriptor sets
        let (descriptor_set_handle, _) = pipeline
            .descriptor_pool
            .create_descriptor_sets(&pipeline.descriptor_set_layout)?;
        let sampler_write_image_info = [*vk::DescriptorImageInfo::builder()
            .image_view(texture_view.handle())
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .sampler(pipeline.sampler.handle())];
        let sampler_writes = [*vk::WriteDescriptorSet::builder()
            .dst_set(
                pipeline
                    .descriptor_pool
                    .descriptor_sets(descriptor_set_handle)?[0]
                    .handle(),
            )
            .dst_binding(0)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(&sampler_write_image_info)];
        pipeline
            .descriptor_pool
            .update_descriptor_sets(&sampler_writes)?;
        let graphics_queue_family_index = queue_family_collection.graphics().index();
        // Create instance buffer
        let instance_buffer = Buffer::new(
            swapchain.context(),
            (SpriteLayer::MAX_SPRITES * std::mem::size_of::<SpriteInstance>()) as u64,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            None,
            None,
        )?
        .with_name("SpriteLayerRenderer::instance_buffer")?;
        {
            let mapped = instance_buffer
                .memory()
                .map_region(0, std::mem::size_of::<SpriteInstance>() as u64)?;
            unsafe {
                *(mapped.ptr() as *mut SpriteInstance) = SpriteInstance {
                    position: (0.0, 0.0),
                    tile_region: TileRegion {
                        left: 0,
                        top: 0,
                        width: 1,
                        height: 1,
                        center_x: 0,
                        center_y: 0,
                    },
                }
            };
        }
        // Create command buffers
        let (command_buffer_handle, command_buffers) = queue_family_collection
            .graphics_mut()
            .command_pools_mut()
            .unwrap()
            .long_term_mut()
            .create_command_buffers(swapchain.images().len() as u32)?;
        for (image_index, image) in swapchain.images().iter().enumerate() {
            let command_buffer_writer = command_buffers[image_index].begin(false, true)?;
            // Transition the swapchain image
            command_buffer_writer.pipeline_barrier(
                initial_state
                    .map(|state| state.0)
                    .unwrap_or(vk::PipelineStageFlags::TOP_OF_PIPE),
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                None,
                None,
                None,
                Some(&[*vk::ImageMemoryBarrier::builder()
                    .image(image.handle())
                    .subresource_range(image.range_color_basic())
                    .old_layout(
                        initial_state
                            .map(|state| state.1)
                            .unwrap_or(vk::ImageLayout::UNDEFINED),
                    )
                    .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .src_access_mask(initial_state.map(|state| state.2).unwrap_or_default())
                    .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)]),
            )?;
            // Start render pass
            {
                let active_pass = command_buffer_writer.begin_render_pass(
                    &pipeline.render_pass,
                    &pipeline.framebuffers[image_index],
                    vk::Rect2D {
                        offset: vk::Offset2D { x: 0, y: 0 },
                        extent: vk::Extent2D {
                            width: swapchain.extent().width,
                            height: swapchain.extent().height,
                        },
                    },
                    &[],
                )?;
                {
                    let active_pipeline = active_pass.bind_graphics_pipeline(&pipeline.pipeline)?;
                    active_pipeline.bind_vertex_buffers(0, &[&instance_buffer], &[0])?;
                    active_pipeline.bind_descriptor_sets(
                        &[&pipeline
                            .descriptor_pool
                            .descriptor_sets(descriptor_set_handle)?[0]],
                        0,
                    )?;
                    active_pipeline.draw(0, 4, 0, 1)?;
                }
            }
        }
        // Return self
        Ok(Self {
            pipeline,
            _descriptor_set_handle: descriptor_set_handle,
            command_buffer_handle,
            _graphics_queue_family_index: graphics_queue_family_index,
            _texture_image: texture_image,
            _texture_view: texture_view,
            _instance_buffer: instance_buffer,
        })
    }
}

impl LayerRenderer for SpriteLayerRenderer {
    fn final_stage(&self) -> vk::PipelineStageFlags {
        vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
    }

    fn final_layout(&self) -> vk::ImageLayout {
        vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL
    }

    fn final_access(&self) -> vk::AccessFlags {
        vk::AccessFlags::COLOR_ATTACHMENT_WRITE
    }

    fn submit_draw(
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
                Some(&[(&wait_for, vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)]),
                Some(&[&self.pipeline.finished_semaphore]),
                signaled_fence,
            )?;
        Ok(&self.pipeline.finished_semaphore)
    }
}

/// The pipeline for a SpriteLayerRenderer, and its associated objects
struct SpritePipeline {
    pipeline: GraphicsPipeline,
    render_pass: RenderPass,
    framebuffers: Vec<Framebuffer>,
    descriptor_set_layout: Rc<RefCell<DescriptorSetLayout>>,
    descriptor_pool: DescriptorPool,
    sampler: Sampler,
    finished_semaphore: Semaphore,
}

impl SpritePipeline {
    fn new(context: &Rc<RefCell<Context>>, swapchain: &Swapchain) -> Result<Self, FennecError> {
        let render_pass_attachments = vec![*vk::AttachmentDescription::builder()
            .format(swapchain.format())
            .samples(vk::SampleCountFlags::TYPE_1)
            .initial_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .final_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .load_op(vk::AttachmentLoadOp::LOAD)
            .store_op(vk::AttachmentStoreOp::STORE)];
        let subpasses = vec![Subpass {
            color_attachments: vec![*vk::AttachmentReference::builder()
                .attachment(0)
                .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)],
            ..Default::default()
        }];
        let render_pass = RenderPass::new(context, &render_pass_attachments, &subpasses)?
            .with_name("SpritePipeline::render_pass")?;
        let framebuffers = swapchain
            .images()
            .iter()
            .enumerate()
            .map(|(index, image)| {
                Framebuffer::new(
                    context,
                    &render_pass,
                    vec![image.view(&image.range_color_basic(), None)?],
                )?
                .with_name(&format!("SpritePipeline::framebuffers[{}]", index))
            })
            .handle_results()?
            .collect();
        let descriptor_set_layout = DescriptorSetLayout::new(
            context,
            1,
            vec![Descriptor {
                shader_stage: vk::ShaderStageFlags::FRAGMENT,
                shader_binding_location: 0,
                descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                count: 1,
            }],
        )?
        .with_name("SpritePipeline::descriptor_set_layout")?;
        let vertex_input_bindings = vec![VertexInputBinding {
            attributes: vec![
                // Position
                VertexInputAttribute {
                    format: AttributeFormat::Float2,
                    offset: 0,
                    shader_binding_location: 0,
                },
                // Region
                VertexInputAttribute {
                    format: AttributeFormat::Int4,
                    offset: 8,
                    shader_binding_location: 1,
                },
            ],
            stride: 24,
            rate: vk::VertexInputRate::INSTANCE,
        }];
        let vertex_shader = ShaderModule::new(
            context,
            &mut ContentEngine::open("sprite.vert", ContentType::ShaderModule)?,
        )?
        .with_name("SpritePipeline::vertex_shader")?;
        let vertex_entry = CString::new(vertex_shader.entry_point())?;
        let fragment_shader = ShaderModule::new(
            context,
            &mut ContentEngine::open("sprite.frag", ContentType::ShaderModule)?,
        )?
        .with_name("SpritePipeline::fragment_shader")?;
        let fragment_entry = CString::new(fragment_shader.entry_point())?;
        let shader_stages = vec![
            *vk::PipelineShaderStageCreateInfo::builder()
                .module(vertex_shader.handle())
                .name(&vertex_entry)
                .stage(vk::ShaderStageFlags::VERTEX),
            *vk::PipelineShaderStageCreateInfo::builder()
                .module(fragment_shader.handle())
                .name(&fragment_entry)
                .stage(vk::ShaderStageFlags::FRAGMENT),
        ];
        let viewports = vec![Viewport {
            width: swapchain.extent().width as f32,
            height: swapchain.extent().height as f32,
            scissor_extent: swapchain.extent(),
            ..Default::default()
        }];
        let pipeline = GraphicsPipeline::new(
            context,
            &render_pass,
            0,
            &[&descriptor_set_layout],
            &vertex_input_bindings,
            vk::PrimitiveTopology::TRIANGLE_STRIP,
            &shader_stages,
            &viewports,
            &GraphicsStates {
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
                ..Default::default()
            },
            None,
        )?
        .with_name("SpritePipeline::pipeline")?;
        let descriptor_pool = DescriptorPool::new(context, &[&descriptor_set_layout], None)?
            .with_name("SpritePipeline::descriptor_pool")?;
        let sampler = Sampler::new(
            context,
            Default::default(),
            Default::default(),
            Default::default(),
            &Default::default(),
        )?
        .with_name("SpritePipeline::sampler")?;
        let finished_semaphore =
            Semaphore::new(context)?.with_name("SpritePipeline::finished_semaphore")?;
        Ok(Self {
            pipeline,
            render_pass,
            framebuffers,
            descriptor_set_layout: Rc::new(RefCell::new(descriptor_set_layout)),
            descriptor_pool,
            sampler,
            finished_semaphore,
        })
    }
}

/// A single sprite instance in a SpriteLayer
#[derive(Debug)]
struct SpriteInstance {
    position: (f32, f32),
    tile_region: TileRegion,
}
