use super::buffer::Buffer;
use super::descriptorpool::DescriptorSet;
use super::framebuffer::Framebuffer;
use super::image::Image;
use super::pipeline::{GraphicsPipeline, Pipeline};
use super::renderpass::RenderPass;
use super::sync::{Fence, Semaphore};
use super::vkobject::{VKHandle, VKObject};
use super::Context;
use crate::cache::{Cache, Handle};
use crate::error::FennecError;
use crate::iteratorext::IteratorResults;
use ash::extensions::khr::Surface;
use ash::version::DeviceV1_0;
use ash::vk;
use ash::{Entry, Instance};
use std::cell::RefCell;
use std::rc::Rc;

/// A collection of general purpose queue families
pub struct QueueFamilyCollection {
    present: QueueFamily,
    graphics: QueueFamily,
    transfer: QueueFamily,
}

impl QueueFamilyCollection {
    /// QueueFamilyCollection factory method
    pub fn new(
        entry: &Entry,
        instance: &Instance,
        device: vk::PhysicalDevice,
        surface: vk::SurfaceKHR,
        families: Vec<vk::QueueFamilyProperties>,
    ) -> Result<Self, FennecError> {
        let surface_loader = Surface::new(entry, instance);
        // Find present family queue
        let present = choose_family(
            "present",
            &families,
            QueueKind::Present,
            |index, _info| unsafe {
                surface_loader.get_physical_device_surface_support(device, index as u32, surface)
            },
        )?;
        // Find graphics family queue
        let graphics = choose_family("graphics", &families, QueueKind::Graphics, |index, info| {
            info.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                && unsafe {
                    surface_loader.get_physical_device_surface_support(
                        device,
                        index as u32,
                        surface,
                    )
                }
        })?;
        // Find transfer family queue
        let transfer = choose_family(
            "transfer",
            &families,
            QueueKind::Transfer,
            |_index, info| info.queue_flags.contains(vk::QueueFlags::TRANSFER),
        )?;
        // Return the queue family collection
        Ok(Self {
            present,
            graphics,
            transfer,
        })
    }

    /// Gets the present queue family
    pub fn present(&self) -> &QueueFamily {
        &self.present
    }

    /// Gets the present queue family
    pub fn present_mut(&mut self) -> &mut QueueFamily {
        &mut self.present
    }

    /// Gets the graphics queue family
    pub fn graphics(&self) -> &QueueFamily {
        &self.graphics
    }

    /// Gets the graphics queue family
    pub fn graphics_mut(&mut self) -> &mut QueueFamily {
        &mut self.graphics
    }

    /// Gets the transfer queue family
    pub fn transfer(&self) -> &QueueFamily {
        &self.transfer
    }

    /// Gets the transfer queue family
    pub fn transfer_mut(&mut self) -> &mut QueueFamily {
        &mut self.transfer
    }

    /// Generate queue priorities
    pub fn queue_priorities(&self) -> Vec<(u32, Vec<f32>)> {
        let mut priorities = vec![
            (self.present().index(), self.present().queue_priorities()),
            (self.graphics().index(), self.graphics().queue_priorities()),
            (self.transfer().index(), self.transfer().queue_priorities()),
        ];
        reduce_family_priorities_to_unique(&mut priorities);
        priorities
    }

    /// Set up queue families
    pub fn setup(&mut self, context: &Rc<RefCell<Context>>) -> Result<(), FennecError> {
        self.present_mut().setup(context)?;
        self.graphics_mut().setup(context)?;
        self.transfer_mut().setup(context)?;
        Ok(())
    }
}

/// Chooses a family that fits specified requirements
fn choose_family<F>(
    name: &str,
    families: &[vk::QueueFamilyProperties],
    kind: QueueKind,
    func: F,
) -> Result<QueueFamily, FennecError>
where
    F: Fn(u32, &vk::QueueFamilyProperties) -> bool,
{
    for (index, ref info) in families.iter().enumerate() {
        let good_queue_family = func(index as u32, *info);
        if good_queue_family {
            return Ok(QueueFamily::new(
                &format!("GraphicsEngine::queue_family_collection.{}", name),
                kind,
                index as u32,
                info.queue_count,
            ));
        }
    }
    Err(FennecError::new(format!(
        "Could not choose a {:?} queue family that meets the requirements",
        kind
    )))
}

/// Takes a list of queue family indices and queue priorities and reduces it
///     down to only unique family indices
fn reduce_family_priorities_to_unique(priorities: &mut Vec<(u32, Vec<f32>)>) {
    let mut first_index = 0;
    while first_index < priorities.len() {
        let mut second_index = first_index + 1;
        while second_index < priorities.len() {
            if priorities[second_index].0 == priorities[first_index].0 {
                if priorities[second_index].1 <= priorities[first_index].1 {
                    priorities.remove(second_index);
                    second_index -= 1;
                } else {
                    priorities.remove(first_index);
                    first_index -= 1;
                    break;
                }
            }
            second_index += 1;
        }
        first_index += 1;
    }
}

/// A Vulkan queue family
pub struct QueueFamily {
    name: String,
    kind: QueueKind,
    index: u32,
    queue_count: u32,
    queues: Option<Vec<Queue>>,
    command_pools: Option<CommandPoolCollection>,
}

impl QueueFamily {
    /// QueueFamily factory method
    fn new(name: &str, kind: QueueKind, index: u32, queue_count: u32) -> Self {
        Self {
            name: String::from(name),
            kind,
            index,
            queue_count,
            queues: None,
            command_pools: None,
        }
    }

    /// Get the queue family index
    pub fn index(&self) -> u32 {
        self.index
    }

    /// Get the kind of queues this queue family creates and owns
    pub fn kind(&self) -> QueueKind {
        self.kind
    }

    /// Get the number of queues
    pub fn queue_count(&self) -> u32 {
        self.queue_count
    }

    /// Get the list of queues
    pub fn queues(&self) -> Option<&Vec<Queue>> {
        self.queues.as_ref()
    }

    /// Get a queue of a specified priority
    pub fn queue_of_priority(&self, priority: f32) -> Option<&Queue> {
        let fractional_index = 1.0 - priority;
        let index = (fractional_index * self.queue_count as f32) as usize;
        let index = index.min(self.queue_count as usize - 1);
        self.queues.as_ref().map(|queues| &queues[index])
    }

    /// Get the queue of index n in a specified priority range
    pub fn queue_n_in_priority_range(
        &self,
        n: usize,
        priority_range: (f32, f32),
    ) -> Option<&Queue> {
        let fractional_index = (
            1.0 - priority_range.0.max(priority_range.1),
            1.0 - priority_range.0.min(priority_range.1),
        );
        let index = (
            (fractional_index.0 * self.queue_count as f32) as usize,
            (fractional_index.1 * self.queue_count as f32) as usize,
        );
        let index = (
            index.0.min(self.queue_count as usize - 1),
            index.1.min(self.queue_count as usize - 1),
        );
        let index = index.0 + (n - index.0) % (index.1 - index.0 + 1);
        self.queues.as_ref().map(|queues| &queues[index])
    }

    /// Get the command pools for this queue family
    pub fn command_pools(&self) -> Option<&CommandPoolCollection> {
        self.command_pools.as_ref()
    }

    /// Get the command pools for this queue family
    pub fn command_pools_mut(&mut self) -> Option<&mut CommandPoolCollection> {
        self.command_pools.as_mut()
    }

    /// Get the queue priorities
    pub fn queue_priorities(&self) -> Vec<f32> {
        let mut priorities = Vec::new();
        for i in 0..self.queue_count {
            priorities.push(1.0 - ((i as f32) / (self.queue_count as f32)));
        }
        priorities
    }

    /// Set up the queue family and its queues
    pub fn setup(&mut self, context: &Rc<RefCell<Context>>) -> Result<(), FennecError> {
        let context_borrowed = context.try_borrow()?;
        self.queues = Some(
            (0..self.queue_count)
                .map(|idx| unsafe {
                    let queue = Queue::new(
                        context,
                        self,
                        context_borrowed
                            .logical_device()
                            .get_device_queue(self.index, idx),
                    )?
                    .with_name(&format!("{}.queues[{}]", self.name, idx))?;
                    Ok(queue)
                })
                .handle_results()?
                .collect(),
        );
        self.command_pools = Some(CommandPoolCollection::new(
            &format!("{}.command_pools", self.name),
            context,
            &self,
        )?);
        Ok(())
    }
}

/// The kind of a queue or queue family
#[derive(Copy, Clone, Debug, PartialEq, Hash)]
pub enum QueueKind {
    Present,
    Graphics,
    Transfer,
    Compute,
}

/// A Vulkan queue
pub struct Queue {
    kind: QueueKind,
    //family_index: u32,
    queue: VKHandle<vk::Queue>,
}

impl Queue {
    /// Queue factory method
    fn new(
        context: &Rc<RefCell<Context>>,
        family: &QueueFamily,
        queue: vk::Queue,
    ) -> Result<Self, FennecError> {
        Ok(Self {
            kind: family.kind(),
            //family_index: family.index(),
            queue: VKHandle::new(context, queue, true),
        })
    }

    /// Gets the kind of queue this is
    pub fn kind(&self) -> QueueKind {
        self.kind
    }

    /*
    /// Gets the queue family index
    pub fn family_index(&self) -> u32 {
        self.family_index
    }*/

    /// Submit a command buffer to the queue
    pub fn submit(
        &self,
        command_buffers: Option<&[&CommandBuffer]>,
        wait_semaphores: Option<&[(&Semaphore, vk::PipelineStageFlags)]>,
        signal_semaphores: Option<&[&Semaphore]>,
        fence: Option<&Fence>,
    ) -> Result<(), FennecError> {
        unsafe {
            let submit_wait_semaphores = match wait_semaphores {
                Some(waits) => waits
                    .iter()
                    .map(|wait| wait.0.handle())
                    .collect::<Vec<vk::Semaphore>>(),
                None => Default::default(),
            };
            let submit_wait_stages = match wait_semaphores {
                Some(waits) => waits
                    .iter()
                    .map(|wait| wait.1)
                    .collect::<Vec<vk::PipelineStageFlags>>(),
                None => Default::default(),
            };
            let submit_signal_semaphores = match signal_semaphores {
                Some(signals) => signals
                    .iter()
                    .map(|signal_semaphore| signal_semaphore.handle())
                    .collect::<Vec<vk::Semaphore>>(),
                None => Default::default(),
            };
            let submit_command_buffers = match command_buffers {
                Some(command_buffers) => command_buffers
                    .iter()
                    .map(|command_buffer| command_buffer.handle())
                    .collect::<Vec<vk::CommandBuffer>>(),
                None => Default::default(),
            };
            self.context().try_borrow()?.logical_device().queue_submit(
                self.handle(),
                &[*vk::SubmitInfo::builder()
                    .wait_semaphores(&submit_wait_semaphores)
                    .wait_dst_stage_mask(&submit_wait_stages)
                    .signal_semaphores(&submit_signal_semaphores)
                    .command_buffers(&submit_command_buffers)],
                fence.map(|e| e.handle()).unwrap_or_default(),
            )
        }?;
        Ok(())
    }

    /// Pause the current thread until the queue is idle
    pub fn wait(&self) -> Result<(), FennecError> {
        unsafe {
            self.context()
                .try_borrow()?
                .logical_device()
                .queue_wait_idle(self.handle())
        }?;
        Ok(())
    }
}

impl VKObject<vk::Queue> for Queue {
    fn wrapped_handle(&self) -> &VKHandle<vk::Queue> {
        &self.queue
    }

    fn wrapped_handle_mut(&mut self) -> &mut VKHandle<vk::Queue> {
        &mut self.queue
    }

    fn object_type() -> vk::DebugReportObjectTypeEXT {
        vk::DebugReportObjectTypeEXT::QUEUE
    }

    fn set_children_names(&mut self) -> Result<(), FennecError> {
        Ok(())
    }
}

/// The collection of command pools owned by a queue family
pub struct CommandPoolCollection {
    transient: CommandPool,
    long_term: CommandPool,
}

impl CommandPoolCollection {
    /// CommandPoolCollection factory method
    fn new(
        name: &str,
        context: &Rc<RefCell<Context>>,
        family: &QueueFamily,
    ) -> Result<Self, FennecError> {
        let transient =
            CommandPool::new(context, family, true)?.with_name(&format!("{}.transient", name))?;
        let long_term =
            CommandPool::new(context, family, false)?.with_name(&format!("{}.long_term", name))?;
        Ok(Self {
            transient,
            long_term,
        })
    }

    /// Get the transient command pool (for command buffers that are short-lived)
    pub fn transient(&self) -> &CommandPool {
        &self.transient
    }

    /// Get the transient command pool (for command buffers that are short-lived)
    pub fn transient_mut(&mut self) -> &mut CommandPool {
        &mut self.transient
    }

    /// Get the long-term command pool (for command buffers that are long-term / reused a lot)
    pub fn long_term(&self) -> &CommandPool {
        &self.long_term
    }

    /// Get the long-term command pool (for command buffers that are long-term / reused a lot)
    pub fn long_term_mut(&mut self) -> &mut CommandPool {
        &mut self.long_term
    }
}

/// A vulkan command pool
pub struct CommandPool {
    command_pool: VKHandle<vk::CommandPool>,
    command_buffers: Cache<Vec<CommandBuffer>>,
    kind: QueueKind,
}

impl CommandPool {
    /// CommandPool factory method
    fn new(
        context: &Rc<RefCell<Context>>,
        family: &QueueFamily,
        transient: bool,
    ) -> Result<Self, FennecError> {
        let create_info = vk::CommandPoolCreateInfo::builder()
            .flags(if transient {
                vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER
                    | vk::CommandPoolCreateFlags::TRANSIENT
            } else {
                vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER
            })
            .queue_family_index(family.index());
        let command_pool = unsafe {
            context
                .try_borrow()?
                .logical_device()
                .create_command_pool(&create_info, None)
        }?;
        Ok(Self {
            command_pool: VKHandle::new(context, command_pool, false),
            command_buffers: Cache::new(),
            kind: family.kind(),
        })
    }

    /// Gets the kind of queues the command pool is used for
    pub fn kind(&self) -> QueueKind {
        self.kind
    }

    /// Creates a set of command buffers
    pub fn create_command_buffers(
        &mut self,
        count: u32,
    ) -> Result<(Handle<Vec<CommandBuffer>>, &mut [CommandBuffer]), FennecError> {
        let handle = self
            .command_buffers
            .insert(CommandBuffer::new(self.context(), self, count)?);
        Ok((handle, self.command_buffers_mut(handle)?))
    }

    /// Destroys a set of command buffers
    pub fn destroy_command_buffers(
        &mut self,
        handle: Handle<Vec<CommandBuffer>>,
    ) -> Result<(), FennecError> {
        let command_buffers = self
            .command_buffers
            .remove(handle)
            .unwrap()
            .into_iter()
            .map(|command_buffer| command_buffer.handle())
            .collect::<Vec<vk::CommandBuffer>>();
        unsafe {
            self.context()
                .try_borrow()?
                .logical_device()
                .free_command_buffers(self.handle(), &command_buffers)
        };
        Ok(())
    }

    /// Gets the set of command buffers pointed to by the specified handle
    pub fn command_buffers(
        &self,
        handle: Handle<Vec<CommandBuffer>>,
    ) -> Result<&[CommandBuffer], FennecError> {
        Ok(self
            .command_buffers
            .get(handle)
            .ok_or_else(|| {
                FennecError::new(format!(
                    "No command buffers exist under handle {:?}",
                    handle
                ))
            })?
            .as_slice())
    }

    /// Gets the set of command buffers pointed to by the specified handle
    pub fn command_buffers_mut(
        &mut self,
        handle: Handle<Vec<CommandBuffer>>,
    ) -> Result<&mut [CommandBuffer], FennecError> {
        Ok(self
            .command_buffers
            .get_mut(handle)
            .ok_or_else(|| {
                FennecError::new(format!(
                    "No command buffers exist under handle {:?}",
                    handle
                ))
            })?
            .as_mut_slice())
    }
}

impl VKObject<vk::CommandPool> for CommandPool {
    fn wrapped_handle(&self) -> &VKHandle<vk::CommandPool> {
        &self.command_pool
    }

    fn wrapped_handle_mut(&mut self) -> &mut VKHandle<vk::CommandPool> {
        &mut self.command_pool
    }

    fn object_type() -> vk::DebugReportObjectTypeEXT {
        vk::DebugReportObjectTypeEXT::COMMAND_POOL
    }

    fn set_children_names(&mut self) -> Result<(), FennecError> {
        let own_name = String::from(self.name());
        for (handle, list) in self.command_buffers.iter_mut() {
            for (index, command_buffer) in list.iter_mut().enumerate() {
                command_buffer.set_name(&format!("{}[{:?}].{}", own_name, handle, index))?;
            }
        }
        Ok(())
    }
}

/// A vulkan command buffer
pub struct CommandBuffer {
    command_buffer: VKHandle<vk::CommandBuffer>,
    writing: bool,
    kind: QueueKind,
}

impl CommandBuffer {
    /// Factory method
    fn new(
        context: &Rc<RefCell<Context>>,
        command_pool: &CommandPool,
        count: u32,
    ) -> Result<Vec<Self>, FennecError> {
        let command_buffers = unsafe {
            let create_info = vk::CommandBufferAllocateInfo::builder()
                .command_buffer_count(count)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_pool(command_pool.handle());
            context
                .try_borrow()?
                .logical_device()
                .allocate_command_buffers(&create_info)?
        };
        Ok(command_buffers
            .iter()
            .map(|buffer| Self {
                command_buffer: VKHandle::new(context, *buffer, false),
                writing: false,
                kind: command_pool.kind(),
            })
            .collect())
    }

    /// Gets the kind of queues the command buffer is to be used in
    pub fn kind(&self) -> QueueKind {
        self.kind
    }

    /// Begins writing to the command buffer
    pub fn begin(
        &mut self,
        used_once: bool,
        simultaneous_use: bool,
    ) -> Result<CommandBufferWriter, FennecError> {
        if self.writing {
            return Err(FennecError::new(
                "CommandBuffer is already being written to",
            ));
        }
        let context = self.context().clone();
        let begin_info = vk::CommandBufferBeginInfo::builder().flags(
            if used_once {
                vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT
            } else {
                Default::default()
            } | if simultaneous_use {
                vk::CommandBufferUsageFlags::SIMULTANEOUS_USE
            } else {
                Default::default()
            },
        );
        unsafe {
            context
                .try_borrow()?
                .logical_device()
                .begin_command_buffer(self.handle(), &begin_info)?;
        }
        self.writing = true;
        Ok(CommandBufferWriter {
            command_buffer: self,
        })
    }

    /// Verifies that the command buffer is for the right type of queue
    pub fn verify_kind(&self, expected_kinds: &[QueueKind]) -> Result<(), FennecError> {
        if expected_kinds.contains(&self.kind()) {
            Ok(())
        } else {
            Err(FennecError::new(&format!(
                "Wrong kind of command buffer ({:?}) - Expected one of {:?}",
                self.kind(),
                expected_kinds
            )))
        }
    }
}

impl VKObject<vk::CommandBuffer> for CommandBuffer {
    fn wrapped_handle(&self) -> &VKHandle<vk::CommandBuffer> {
        &self.command_buffer
    }

    fn wrapped_handle_mut(&mut self) -> &mut VKHandle<vk::CommandBuffer> {
        &mut self.command_buffer
    }

    fn object_type() -> vk::DebugReportObjectTypeEXT {
        vk::DebugReportObjectTypeEXT::COMMAND_BUFFER
    }

    fn set_children_names(&mut self) -> Result<(), FennecError> {
        Ok(())
    }
}

/// Writers to write to a command buffer
pub struct CommandBufferWriter<'a> {
    command_buffer: &'a mut CommandBuffer,
}

impl<'a> CommandBufferWriter<'a> {
    /// Consumes the command buffer writer, ending writing to the command buffer
    pub fn end(self) {}

    /// Inserts a pipeline barrier
    pub fn pipeline_barrier(
        &self,
        src_stage: vk::PipelineStageFlags,
        dst_stage: vk::PipelineStageFlags,
        dependency_flags: Option<vk::DependencyFlags>,
        memory_barriers: Option<&[vk::MemoryBarrier]>,
        buffer_memory_barriers: Option<&[vk::BufferMemoryBarrier]>,
        image_memory_barriers: Option<&[vk::ImageMemoryBarrier]>,
    ) -> Result<(), FennecError> {
        self.command_buffer.verify_kind(&[
            QueueKind::Transfer,
            QueueKind::Graphics,
            QueueKind::Compute,
        ])?;
        unsafe {
            self.command_buffer
                .context()
                .try_borrow()?
                .logical_device()
                .cmd_pipeline_barrier(
                    self.command_buffer.handle(),
                    src_stage,
                    dst_stage,
                    dependency_flags.unwrap_or_default(),
                    memory_barriers.unwrap_or_else(|| &[]),
                    buffer_memory_barriers.unwrap_or_else(|| &[]),
                    image_memory_barriers.unwrap_or_else(|| &[]),
                );
            Ok(())
        }
    }

    /// Clears the color of an image
    /// ``image``: The image to clear
    /// ``layout``: The layout of the image
    /// ``clear_color``: The color to clear with
    /// ``ranges``: The image subresource ranges to clear
    pub fn clear_color_image(
        &self,
        image: &impl Image,
        layout: vk::ImageLayout,
        clear_color: &vk::ClearColorValue,
        ranges: &[vk::ImageSubresourceRange],
    ) -> Result<(), FennecError> {
        self.command_buffer
            .verify_kind(&[QueueKind::Graphics, QueueKind::Compute])?;
        unsafe {
            self.command_buffer
                .context()
                .try_borrow()?
                .logical_device()
                .cmd_clear_color_image(
                    self.command_buffer.handle(),
                    image.image_handle().handle(),
                    layout,
                    clear_color,
                    ranges,
                );
            Ok(())
        }
    }

    /// Begins a render pass, returning an ActiveRenderPass representing it
    pub fn begin_render_pass(
        &self,
        render_pass: &RenderPass,
        framebuffer: &Framebuffer,
        render_area: vk::Rect2D,
        clear_values: &[vk::ClearValue],
    ) -> Result<ActiveRenderPass, FennecError> {
        self.command_buffer.verify_kind(&[QueueKind::Graphics])?;
        let begin_info = vk::RenderPassBeginInfo::builder()
            .render_pass(render_pass.handle())
            .framebuffer(framebuffer.handle())
            .render_area(render_area)
            .clear_values(clear_values);
        unsafe {
            self.command_buffer
                .context()
                .try_borrow()?
                .logical_device()
                .cmd_begin_render_pass(
                    self.command_buffer.handle(),
                    &begin_info,
                    Default::default(),
                );
            Ok(ActiveRenderPass {
                command_buffer_writer: self,
            })
        }
    }

    /// Copies regions of a buffer's contents to an image
    pub unsafe fn copy_buffer_to_image(
        &self,
        source: &Buffer,
        destination: &impl Image,
        destination_layout: vk::ImageLayout,
        regions: &[vk::BufferImageCopy],
    ) -> Result<(), FennecError> {
        self.command_buffer.verify_kind(&[
            QueueKind::Transfer,
            QueueKind::Graphics,
            QueueKind::Compute,
        ])?;
        // Check image regions
        for region in regions {
            // TODO: Check buffer region as well
            // TODO: and then remove "unsafe" if it is safe after
            destination.verify_region_is_inside(region.image_offset, region.image_extent)?;
        }
        // Do the copy
        //unsafe {
        self.command_buffer
            .context()
            .try_borrow()?
            .logical_device()
            .cmd_copy_buffer_to_image(
                self.command_buffer.handle(),
                source.handle(),
                destination.image_handle().handle(),
                destination_layout,
                regions,
            );
        //}
        Ok(())
    }
}

impl<'a> Drop for CommandBufferWriter<'a> {
    fn drop(&mut self) {
        // Stop writing to the associated command buffer when this is dropped
        self.command_buffer.writing = false;
        unsafe {
            self.command_buffer
                .context()
                .borrow()
                .logical_device()
                .end_command_buffer(self.command_buffer.handle())
                .unwrap();
        }
    }
}

/// Wrapper around a CommandBufferWriter that is writing inside of a render pass\
/// Enables writing commands that require an active render pass
pub struct ActiveRenderPass<'a> {
    command_buffer_writer: &'a CommandBufferWriter<'a>,
}

impl<'a> ActiveRenderPass<'a> {
    /// Consume the ActiveRenderPass, ending the render pass
    pub fn end(self) {}

    /// Bind a graphics pipeline
    pub fn bind_graphics_pipeline(
        &self,
        pipeline: &'a GraphicsPipeline,
    ) -> Result<ActiveGraphicsPipeline, FennecError> {
        let command_buffer_handle = self.command_buffer_writer.command_buffer.handle();
        unsafe {
            self.command_buffer_writer
                .command_buffer
                .context()
                .try_borrow()?
                .logical_device()
                .cmd_bind_pipeline(
                    command_buffer_handle,
                    vk::PipelineBindPoint::GRAPHICS,
                    pipeline.handle(),
                );
            // TODO: Start pipeline usage benchmark
            Ok(ActiveGraphicsPipeline {
                pipeline,
                active_render_pass: self,
            })
        }
    }
}

impl<'a> Drop for ActiveRenderPass<'a> {
    fn drop(&mut self) {
        // End the render pass when this is dropped
        unsafe {
            self.command_buffer_writer
                .command_buffer
                .context()
                .borrow()
                .logical_device()
                .cmd_end_render_pass(self.command_buffer_writer.command_buffer.handle());
        }
    }
}

/// Wrapper around an ActiveRenderPass that has a graphics pipeline bound\
/// Enables writing commands that require an active graphics pipeline
pub struct ActiveGraphicsPipeline<'a> {
    pipeline: &'a GraphicsPipeline,
    active_render_pass: &'a ActiveRenderPass<'a>,
}

impl<'a> ActiveGraphicsPipeline<'a> {
    /// Consume the ActiveRenderPass, ending the render pass
    pub fn end(self) {}

    /// Bind an index buffer
    pub fn bind_index_buffer(
        &self,
        buffer: &Buffer,
        offset_bytes: u64,
        index_type: vk::IndexType,
    ) -> Result<(), FennecError> {
        unsafe {
            self.active_render_pass
                .command_buffer_writer
                .command_buffer
                .context()
                .try_borrow()?
                .logical_device()
                .cmd_bind_index_buffer(
                    self.active_render_pass
                        .command_buffer_writer
                        .command_buffer
                        .handle(),
                    buffer.handle(),
                    offset_bytes,
                    index_type,
                );
            Ok(())
        }
    }

    /// Bind a descriptor set
    pub fn bind_descriptor_sets(
        &self,
        descriptor_sets: &[&DescriptorSet],
        first_set: u32,
    ) -> Result<(), FennecError> {
        unsafe {
            let descriptor_sets = descriptor_sets
                .iter()
                .map(|set| set.handle())
                .collect::<Vec<vk::DescriptorSet>>();
            self.active_render_pass
                .command_buffer_writer
                .command_buffer
                .context()
                .try_borrow()?
                .logical_device()
                .cmd_bind_descriptor_sets(
                    self.active_render_pass
                        .command_buffer_writer
                        .command_buffer
                        .handle(),
                    vk::PipelineBindPoint::GRAPHICS,
                    self.pipeline.layout().handle(),
                    first_set,
                    &descriptor_sets,
                    &[],
                );
            Ok(())
        }
    }

    /// Dispatch a draw
    pub fn draw(
        &self,
        first_vertex: u32,
        vertex_count: u32,
        first_instance: u32,
        instance_count: u32,
    ) -> Result<(), FennecError> {
        if vertex_count == 0 {
            return Err(FennecError::new("Vertex count was 0"));
        }
        if instance_count == 0 {
            return Err(FennecError::new("Instance count was 0"));
        }
        unsafe {
            self.active_render_pass
                .command_buffer_writer
                .command_buffer
                .context()
                .try_borrow()?
                .logical_device()
                .cmd_draw(
                    self.active_render_pass
                        .command_buffer_writer
                        .command_buffer
                        .handle(),
                    vertex_count,
                    instance_count,
                    first_vertex,
                    first_instance,
                );
            Ok(())
        }
    }

    /// Dispatch an indexed draw
    pub fn draw_indexed(
        &self,
        first_index: u32,
        index_count: u32,
        vertex_offset: i32,
        first_instance: u32,
        instance_count: u32,
    ) -> Result<(), FennecError> {
        if index_count == 0 {
            return Err(FennecError::new("Index count was 0"));
        }
        if instance_count == 0 {
            return Err(FennecError::new("Instance count was 0"));
        }
        unsafe {
            self.active_render_pass
                .command_buffer_writer
                .command_buffer
                .context()
                .try_borrow()?
                .logical_device()
                .cmd_draw_indexed(
                    self.active_render_pass
                        .command_buffer_writer
                        .command_buffer
                        .handle(),
                    index_count,
                    instance_count,
                    first_index,
                    vertex_offset,
                    first_instance,
                );
            Ok(())
        }
    }
}

impl<'a> Drop for ActiveGraphicsPipeline<'a> {
    fn drop(&mut self) {
        // TODO: End pipeline usage benchmark
    }
}
