use super::memory::{Memory, MemoryMap};
use super::queue::QueueFamily;
use super::vkobject::{VKHandle, VKObject};
use super::Context;
use crate::error::FennecError;
use ash::version::DeviceV1_0;
use ash::vk;
use std::cell::RefCell;
use std::rc::Rc;

/// A Vulkan buffer
pub struct Buffer {
    buffer: VKHandle<vk::Buffer>,
    memory: Memory,
    size: u64,
}

impl Buffer {
    /// General buffer factory method
    pub fn new(
        context: &Rc<RefCell<Context>>,
        size: u64,
        usage: vk::BufferUsageFlags,
        memory_flags: vk::MemoryPropertyFlags,
        simultaneous_use: Option<&[&QueueFamily]>,
        flags: Option<vk::BufferCreateFlags>,
    ) -> Result<Self, FennecError> {
        let context_borrowed = context.try_borrow()?;
        let logical_device = context_borrowed.logical_device();
        // Set buffer create info
        let queue_family_indices = simultaneous_use
            .unwrap_or_else(|| &[])
            .iter()
            .map(|family| family.index())
            .collect::<Vec<u32>>();
        let create_info = vk::BufferCreateInfo::builder()
            .flags(flags.unwrap_or_default())
            .size(size)
            .usage(usage)
            .sharing_mode(if simultaneous_use.is_some() {
                vk::SharingMode::CONCURRENT
            } else {
                vk::SharingMode::EXCLUSIVE
            })
            .queue_family_indices(&queue_family_indices);
        // Create buffer
        let buffer = unsafe { logical_device.create_buffer(&create_info, None) }?;
        // Create device memory
        let memory = Memory::new(
            context,
            unsafe { logical_device.get_buffer_memory_requirements(buffer) },
            memory_flags,
        )?;
        // Bind memory to buffer
        unsafe { logical_device.bind_buffer_memory(buffer, *memory.handle().handle(), 0) }?;
        // Return buffer
        Ok(Self {
            buffer: VKHandle::new(context, buffer, false),
            memory,
            size,
        })
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn map(&mut self, offset: u64, size: u64) -> Result<MemoryMap, FennecError> {
        self.memory.map(offset, size)
    }
}

impl VKObject<vk::Buffer> for Buffer {
    fn handle(&self) -> &VKHandle<vk::Buffer> {
        &self.buffer
    }

    fn handle_mut(&mut self) -> &mut VKHandle<vk::Buffer> {
        &mut self.buffer
    }

    fn object_type() -> vk::DebugReportObjectTypeEXT {
        vk::DebugReportObjectTypeEXT::BUFFER
    }

    fn set_children_names(&mut self) -> Result<(), FennecError> {
        self.memory.set_name(&format!("{}.memory", self.name()))?;
        Ok(())
    }
}
