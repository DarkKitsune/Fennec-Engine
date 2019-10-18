use super::image::Image;
use super::memory::Memory;
use super::queuefamily::QueueFamily;
use super::vkobject::{VKHandle, VKObject};
use super::Context;
use crate::error::FennecError;
use ash::version::DeviceV1_0;
use ash::vk;
use std::cell::RefCell;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::ptr;
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

    /// Create a buffer containing length number of bytes read from a source
    pub unsafe fn from_bytes(
        context: &Rc<RefCell<Context>>,
        bytes: &[u8],
        length: usize,
        usage: vk::BufferUsageFlags,
        simultaneous_use: Option<&[&QueueFamily]>,
        flags: Option<vk::BufferCreateFlags>,
    ) -> Result<Self, FennecError> {
        let mut buffer = Self::new(
            context,
            length as u64,
            usage,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            simultaneous_use,
            flags,
        )?;
        {
            let mapped_buffer = buffer.memory_mut().map_all()?;
            ptr::copy_nonoverlapping(bytes.as_ptr(), mapped_buffer.ptr() as *mut u8, length);
        }
        Ok(buffer)
    }

    /// Create a buffer containing length number of bytes read from a source
    pub fn from_read(
        context: &Rc<RefCell<Context>>,
        bytes: impl Read,
        length: u64,
        usage: vk::BufferUsageFlags,
        simultaneous_use: Option<&[&QueueFamily]>,
        flags: Option<vk::BufferCreateFlags>,
    ) -> Result<Self, FennecError> {
        let mut buffer = Self::new(
            context,
            length,
            usage,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            simultaneous_use,
            flags,
        )?;
        {
            let mapped_buffer = buffer.memory_mut().map_all()?;
            let mut source = Vec::new();
            let read_bytes = bytes.take(length).read_to_end(&mut source)?;
            unsafe {
                ptr::copy_nonoverlapping(
                    &source[0] as *const u8,
                    mapped_buffer.ptr() as *mut u8,
                    read_bytes,
                )
            };
        }
        Ok(buffer)
    }

    /// Create a buffer containing the contents of a file
    pub fn from_file(
        context: &Rc<RefCell<Context>>,
        file: &mut File,
        usage: vk::BufferUsageFlags,
        simultaneous_use: Option<&[&QueueFamily]>,
        flags: Option<vk::BufferCreateFlags>,
    ) -> Result<Self, FennecError> {
        let original_position = file.seek(SeekFrom::Current(0))?;
        let end = file.seek(SeekFrom::End(0))?;
        file.seek(SeekFrom::Start(original_position))?;
        let length = end - original_position;
        Self::from_read(context, file, length, usage, simultaneous_use, flags)
    }

    /// Gets the buffer size in bytes
    pub fn size(&self) -> u64 {
        self.size
    }

    /// Gets the device memory backing the buffer
    pub fn memory(&self) -> &Memory {
        &self.memory
    }

    /// Gets the device memory backing the buffer
    pub fn memory_mut(&mut self) -> &mut Memory {
        &mut self.memory
    }

    /// Generates vk::BufferImageCopy describing a copy from the buffer to an entire image.\
    /// Used in CommandBuffer::copy_buffer_to_image()
    pub fn copy_to_image(
        offset: u64,
        destination: &impl Image,
        aspects: vk::ImageAspectFlags,
        mip_level: u32,
    ) -> vk::BufferImageCopy {
        *vk::BufferImageCopy::builder()
            .buffer_offset(offset)
            .buffer_row_length(destination.extent().width)
            .buffer_image_height(destination.extent().height)
            .image_subresource(destination.layers(aspects, 0, destination.layer_count(), mip_level))
            .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
            .image_extent(destination.extent())
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
