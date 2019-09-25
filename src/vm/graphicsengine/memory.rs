use super::vkobject::{VKHandle, VKObject};
use super::Context;
use crate::error::FennecError;
use ash::version::{DeviceV1_0, InstanceV1_0};
use ash::vk;
use std::cell::RefCell;
use std::rc::Rc;
pub use vk::{
    Extent2D, Format, ImageCreateFlags, ImageLayout, ImageTiling, ImageUsageFlags, SampleCountFlags,
};

/// A portion of memory allocated on the graphics device
pub struct Memory {
    memory: VKHandle<vk::DeviceMemory>,
}

impl Memory {
    /// Factory method for Memory
    pub fn new(
        context: &Rc<RefCell<Context>>,
        memory_reqs: vk::MemoryRequirements,
    ) -> Result<Self, FennecError> {
        let context_borrowed = context.try_borrow()?;
        let logical_device = context_borrowed.logical_device();
        // Create memory allocate info
        let allocate_info = vk::MemoryAllocateInfo::builder()
            .memory_type_index(get_memory_type_index(
                context_borrowed.instance(),
                context_borrowed.physical_device(),
                memory_reqs.memory_type_bits,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
            )?)
            .allocation_size(memory_reqs.size)
            .build();
        // Allocate memory
        let memory = unsafe { logical_device.allocate_memory(&allocate_info, None) }?;
        // Return memory
        Ok(Self {
            memory: VKHandle::new(context, memory, false),
        })
    }
}

impl VKObject<vk::DeviceMemory> for Memory {
    fn handle(&self) -> &VKHandle<vk::DeviceMemory> {
        &self.memory
    }

    fn handle_mut(&mut self) -> &mut VKHandle<vk::DeviceMemory> {
        &mut self.memory
    }

    fn object_type() -> vk::DebugReportObjectTypeEXT {
        vk::DebugReportObjectTypeEXT::DEVICE_MEMORY
    }
}

/// Finds the index of a memory type that fits the given requirements
fn get_memory_type_index(
    instance: &ash::Instance,
    physical_device: &vk::PhysicalDevice,
    type_bits: u32,
    properties: vk::MemoryPropertyFlags,
) -> Result<u32, FennecError> {
    // Enumerate physical device memory properties
    let memory_properties =
        unsafe { instance.get_physical_device_memory_properties(*physical_device) };
    // Count number of memory properties
    let count = memory_properties.memory_type_count;
    // Return index of memory properties matching required properties
    memory_properties
        .memory_types
        .iter()
        .take(count as usize)
        .enumerate()
        .find(|e| (e.1).property_flags == properties)
        .map(|e| e.0 as u32)
        .ok_or_else(|| {
            FennecError::new(format!(
                "Memory type not available: type_bits={} properties={:?}",
                type_bits, properties
            ))
        })
}
