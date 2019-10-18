use super::vkobject::{VKHandle, VKObject};
use super::Context;
use crate::error::FennecError;
use ash::version::{DeviceV1_0, InstanceV1_0};
use ash::vk;
use std::cell::RefCell;
use std::ffi::c_void;
use std::rc::Rc;

/// A portion of memory allocated on the graphics device
pub struct Memory {
    memory: VKHandle<vk::DeviceMemory>,
    memory_flags: vk::MemoryPropertyFlags,
    size: u64,
}

impl Memory {
    /// Factory method for Memory
    pub fn new(
        context: &Rc<RefCell<Context>>,
        memory_reqs: vk::MemoryRequirements,
        memory_flags: vk::MemoryPropertyFlags,
    ) -> Result<Self, FennecError> {
        let context_borrowed = context.try_borrow()?;
        let logical_device = context_borrowed.logical_device();
        // Create memory allocate info
        let allocate_info = vk::MemoryAllocateInfo::builder()
            .memory_type_index(get_memory_type_index(
                context_borrowed.instance(),
                *context_borrowed.physical_device(),
                memory_reqs.memory_type_bits,
                memory_flags,
            )?)
            .allocation_size(memory_reqs.size);
        // Allocate memory
        let memory = unsafe { logical_device.allocate_memory(&allocate_info, None) }?;
        // Return memory
        Ok(Self {
            memory: VKHandle::new(context, memory, false),
            memory_flags,
            size: memory_reqs.size,
        })
    }

    /// Gets the allocated size of the memory
    pub fn size(&self) -> u64 {
        self.size
    }

    /// Maps a region of the memory to host memory for writing
    pub fn map_region(&mut self, offset: u64, size: u64) -> Result<MemoryMap, FennecError> {
        if !self.mappable() {
            return Err(FennecError::new(format!(
                "Cannot map {} as it is either protected or host-invisible",
                self.name()
            )));
        }
        if offset + size > self.size() {
            return Err(FennecError::new(format!(
                "Region (offset={} size={}) is not within {}'s mappable range (size={})",
                offset,
                size,
                self.name(),
                self.size()
            )));
        }
        let ptr = unsafe {
            self.context().try_borrow()?.logical_device().map_memory(
                *self.handle().handle(),
                offset,
                size,
                Default::default(),
            )?
        };
        Ok(MemoryMap {
            context: self.context().clone(),
            memory: self,
            ptr,
        })
    }

    pub fn map_all(&mut self) -> Result<MemoryMap, FennecError> {
        self.map_region(0, self.size())
    }

    /// Gets whether the memory is mappable to host memory
    pub fn mappable(&self) -> bool {
        self.memory_flags & vk::MemoryPropertyFlags::HOST_VISIBLE
            == vk::MemoryPropertyFlags::HOST_VISIBLE
            && self.memory_flags & vk::MemoryPropertyFlags::PROTECTED
                != vk::MemoryPropertyFlags::PROTECTED
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

    fn set_children_names(&mut self) -> Result<(), FennecError> {
        Ok(())
    }
}

/// Finds the index of a memory type that fits the given requirements
fn get_memory_type_index(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    type_bits: u32,
    properties: vk::MemoryPropertyFlags,
) -> Result<u32, FennecError> {
    // Enumerate physical device memory properties
    let memory_properties =
        unsafe { instance.get_physical_device_memory_properties(physical_device) };
    // Count number of memory properties
    let count = memory_properties.memory_type_count;
    // Return index of memory properties matching required properties
    memory_properties
        .memory_types
        .iter()
        .take(count as usize)
        .enumerate()
        .find(|e| {
            type_bits & 2u32.pow(e.0 as u32) == 2u32.pow(e.0 as u32)
                && (e.1).property_flags & properties == properties
        })
        .map(|e| e.0 as u32)
        .ok_or_else(|| {
            FennecError::new(format!(
                "Memory type not available: type_bits={} properties={:?}",
                type_bits, properties
            ))
        })
}

/// Represents a region of device memory mapped to host memory
pub struct MemoryMap<'a> {
    context: Rc<RefCell<Context>>,
    memory: &'a mut Memory,
    ptr: *mut c_void,
}

impl MemoryMap<'_> {
    /// Unmaps the memory region and consume this MemoryMap object
    pub fn unmap(self) {}

    // TODO: v get rid of this unsafe garbage and replace it with safer writing methods?
    /// Gets the pointer to the beginning of the memory region.\
    /// This function is ``unsafe`` as the pointer will not prevent writing outside of the region,
    /// which leads to undefined behavior.
    pub unsafe fn ptr(&self) -> *mut c_void {
        self.ptr
    }
}

impl Drop for MemoryMap<'_> {
    fn drop(&mut self) {
        unsafe {
            self.context
                .try_borrow()
                .unwrap()
                .logical_device()
                .unmap_memory(*self.memory.handle().handle())
        }
    }
}
