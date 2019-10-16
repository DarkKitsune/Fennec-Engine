use super::buffer::Buffer;
use super::cache::{Cache, Handle};
use super::vkobject::{VKHandle, VKObject};
use super::Context;
use crate::error::FennecError;
use ash::version::DeviceV1_0;
use ash::vk;
use std::cell::RefCell;
use std::rc::Rc;

/// A descriptor pool from which descriptor sets are create from
pub struct DescriptorPool {
    descriptor_pool: VKHandle<vk::DescriptorPool>,
    descriptor_sets: Cache<Vec<DescriptorSet>>,
}

impl DescriptorPool {
    /// Factory method
    pub fn new(
        context: &Rc<RefCell<Context>>,
        set_layouts: &[&DescriptorSetLayout],
        advanced_settings: Option<AdvancedDescriptorPoolSettings>,
    ) -> Result<Self, FennecError> {
        let advanced_settings = advanced_settings.unwrap_or_default();
        // Set create info
        let pool_sizes = set_layouts
            .iter()
            .map(|alloc| {
                alloc.descriptors.iter().map(move |descriptor| {
                    *vk::DescriptorPoolSize::builder()
                        .ty(descriptor.descriptor_type)
                        .descriptor_count(descriptor.count * alloc.count)
                })
            })
            .flatten()
            .collect::<Vec<vk::DescriptorPoolSize>>();
        let pool_sizes = {
            let mut uniques = Vec::new();
            for pool_size in pool_sizes.iter() {
                if uniques
                    .iter()
                    .find(|unique: &&vk::DescriptorPoolSize| unique.ty == pool_size.ty)
                    .is_none()
                {
                    let counts = pool_sizes.iter().filter_map(|pool_size2| {
                        if pool_size2.ty == pool_size.ty {
                            Some(pool_size2.descriptor_count)
                        } else {
                            None
                        }
                    });
                    uniques.push(
                        *vk::DescriptorPoolSize::builder()
                            .ty(pool_size.ty)
                            .descriptor_count(counts.sum()),
                    );
                }
            }
            uniques
        };
        let create_info = vk::DescriptorPoolCreateInfo::builder()
            .flags(if advanced_settings.update_after_bind.unwrap_or_default() {
                vk::DescriptorPoolCreateFlags::UPDATE_AFTER_BIND_EXT
            } else {
                Default::default()
            })
            .max_sets(set_layouts.iter().map(|alloc| alloc.count).sum())
            .pool_sizes(&pool_sizes);
        // Create descriptor pool
        let descriptor_pool = unsafe {
            context
                .try_borrow()?
                .logical_device()
                .create_descriptor_pool(&create_info, None)
        }?;
        // Return descriptor pool
        Ok(Self {
            descriptor_pool: VKHandle::new(context, descriptor_pool, false),
            descriptor_sets: Cache::new(),
        })
    }

    /// Creates a set of descriptor sets
    pub fn create_descriptor_sets(
        &mut self,
        layout: &Rc<RefCell<DescriptorSetLayout>>,
    ) -> Result<(Handle<Vec<DescriptorSet>>, &mut [DescriptorSet]), FennecError> {
        let own_name = String::from(self.name());
        let descriptor_sets = DescriptorSet::new(self.context(), self, layout)?;
        let handle = self.descriptor_sets.insert(descriptor_sets);
        let descriptor_sets = self.descriptor_sets_mut(handle)?;
        for (index, set) in descriptor_sets.iter_mut().enumerate() {
            set.set_name(&format!("{}[{:?}].{}", own_name, handle, index))?;
        }
        Ok((handle, descriptor_sets))
    }

    /// Gets the set of descriptor sets pointed to by the specified handle
    pub fn descriptor_sets(
        &self,
        handle: Handle<Vec<DescriptorSet>>,
    ) -> Result<&[DescriptorSet], FennecError> {
        Ok(self
            .descriptor_sets
            .get(handle)
            .ok_or_else(|| {
                FennecError::new(format!(
                    "No descriptor sets exist under handle {:?}",
                    handle
                ))
            })?
            .as_slice())
    }

    /// Gets the set of descriptor sets pointed to by the specified handle
    pub fn descriptor_sets_mut(
        &mut self,
        handle: Handle<Vec<DescriptorSet>>,
    ) -> Result<&mut [DescriptorSet], FennecError> {
        Ok(self
            .descriptor_sets
            .get_mut(handle)
            .ok_or_else(|| {
                FennecError::new(format!(
                    "No descriptor sets exist under handle {:?}",
                    handle
                ))
            })?
            .as_mut_slice())
    }

    /// Update descriptor sets
    pub fn update_descriptor_sets(
        &self,
        writes: &[vk::WriteDescriptorSet],
    ) -> Result<(), FennecError> {
        let copies = vec![];
        unsafe {
            self.context()
                .try_borrow()?
                .logical_device()
                .update_descriptor_sets(writes, &copies);
        }
        Ok(())
    }
}

impl VKObject<vk::DescriptorPool> for DescriptorPool {
    fn handle(&self) -> &VKHandle<vk::DescriptorPool> {
        &self.descriptor_pool
    }

    fn handle_mut(&mut self) -> &mut VKHandle<vk::DescriptorPool> {
        &mut self.descriptor_pool
    }

    fn object_type() -> vk::DebugReportObjectTypeEXT {
        vk::DebugReportObjectTypeEXT::DESCRIPTOR_POOL
    }

    fn set_children_names(&mut self) -> Result<(), FennecError> {
        let own_name = String::from(self.name());
        for (handle, list) in self.descriptor_sets.iter_mut() {
            for (index, descriptor_set) in list.iter_mut().enumerate() {
                descriptor_set.set_name(&format!("{}[{:?}].{}", own_name, handle, index))?;
            }
        }
        Ok(())
    }
}

/// Advanced settings to be used in descriptor pool factory methods
#[derive(Default, Copy, Clone)]
pub struct AdvancedDescriptorPoolSettings {
    /// Allow use of DescriptorPoolCreateFlags::UPDATE_AFTER_BIND_POOL_EXT *(default=false)*
    pub update_after_bind: Option<bool>,
}

/// A descriptor set
pub struct DescriptorSet {
    descriptor_set: VKHandle<vk::DescriptorSet>,
    layout: Rc<RefCell<DescriptorSetLayout>>,
}

impl DescriptorSet {
    /// Factory method
    fn new(
        context: &Rc<RefCell<Context>>,
        pool: &DescriptorPool,
        layout: &Rc<RefCell<DescriptorSetLayout>>,
    ) -> Result<Vec<Self>, FennecError> {
        let layout_borrowed = layout.try_borrow()?;
        // Make a vector of layout.count copies of the layout's handle
        let set_layouts = (0..layout_borrowed.count)
            .map(|_index| *layout_borrowed.handle().handle())
            .collect::<Vec<vk::DescriptorSetLayout>>();
        // Set create info
        let create_info = vk::DescriptorSetAllocateInfo::builder()
            .set_layouts(&set_layouts)
            .descriptor_pool(*pool.handle().handle());
        // Return vector of descriptor sets
        Ok(unsafe {
            context
                .try_borrow()?
                .logical_device()
                .allocate_descriptor_sets(&create_info)
        }?
        .iter()
        .map(|&descriptor_set| Self {
            descriptor_set: VKHandle::new(context, descriptor_set, true),
            layout: layout.clone(),
        })
        .collect())
    }

    pub fn layout(&self) -> &Rc<RefCell<DescriptorSetLayout>> {
        &self.layout
    }

    /// Creates a vk::WriteDescriptorSet describing a buffer write to a descriptor in the set
    pub fn write_uniform_buffers(
        &self,
        descriptor_index: u32,
        start: u32,
        buffer_writes: &[BufferWrite],
    ) -> Result<vk::WriteDescriptorSet, FennecError> {
        let buffer_writes = buffer_writes
            .iter()
            .map(|write| {
                *vk::DescriptorBufferInfo::builder()
                    .buffer(*write.buffer.handle().handle())
                    .offset(write.offset)
                    .range(write.length)
            })
            .collect::<Vec<vk::DescriptorBufferInfo>>();
        // Check arguments
        self.write_argument_check(
            descriptor_index,
            start,
            buffer_writes.len() as u32,
            vk::DescriptorType::UNIFORM_BUFFER,
        )?;
        // Return write info
        Ok(*vk::WriteDescriptorSet::builder()
            .dst_set(*self.handle().handle())
            .dst_binding(descriptor_index)
            .dst_array_element(start)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .buffer_info(&buffer_writes))
    }

    /// Used to check the arguments passed to write_* functions
    fn write_argument_check(
        &self,
        descriptor_index: u32,
        start: u32,
        count: u32,
        expected_descriptor_type: vk::DescriptorType,
    ) -> Result<(), FennecError> {
        let layout = self.layout.try_borrow()?;
        // Must be a valid descriptor index
        if descriptor_index >= layout.descriptors.len() as u32 {
            return Err(FennecError::new(&format!(
                "Index {} is not a valid descriptor index in {}",
                descriptor_index,
                self.name()
            )));
        }
        // start + count must be inside the descriptor's range
        if start + count > layout.descriptors[descriptor_index as usize].count {
            return Err(FennecError::new(&format!(
                "Range (start={}, count={}) is not within the range of descriptor {} in {}",
                start,
                count,
                descriptor_index,
                self.name()
            )));
        }
        let descriptor_type = layout.descriptors[descriptor_index as usize].descriptor_type;
        if descriptor_type != expected_descriptor_type {
            return Err(FennecError::new(&format!(
                "Expected descriptor's type to be {:?} but it was {:?}",
                expected_descriptor_type, descriptor_type
            )));
        }
        Ok(())
    }
}

impl VKObject<vk::DescriptorSet> for DescriptorSet {
    fn handle(&self) -> &VKHandle<vk::DescriptorSet> {
        &self.descriptor_set
    }

    fn handle_mut(&mut self) -> &mut VKHandle<vk::DescriptorSet> {
        &mut self.descriptor_set
    }

    fn object_type() -> vk::DebugReportObjectTypeEXT {
        vk::DebugReportObjectTypeEXT::DESCRIPTOR_SET
    }

    fn set_children_names(&mut self) -> Result<(), FennecError> {
        Ok(())
    }
}

/// Describes a write to a buffer
pub struct BufferWrite<'a> {
    pub buffer: &'a Buffer,
    pub offset: u64,
    pub length: u64,
}

/// Describes the layout for a type of descriptor set from a descriptor pool
pub struct DescriptorSetLayout {
    /// The Vulkan descriptor set layout handle
    layout: VKHandle<vk::DescriptorSetLayout>,
    /// Number of this type of descriptor set to allocate resources for
    count: u32,
    /// The list of descriptors in the descriptor set
    descriptors: Vec<Descriptor>,
}

impl DescriptorSetLayout {
    /// Factory method
    pub fn new(
        context: &Rc<RefCell<Context>>,
        count: u32,
        descriptors: Vec<Descriptor>,
    ) -> Result<Self, FennecError> {
        // Set binding infos
        let bindings = descriptors
            .iter()
            .map(|descriptor| {
                *vk::DescriptorSetLayoutBinding::builder()
                    .stage_flags(descriptor.shader_stage)
                    .binding(descriptor.shader_binding_location)
                    .descriptor_type(descriptor.descriptor_type)
                    .descriptor_count(descriptor.count)
            })
            .collect::<Vec<vk::DescriptorSetLayoutBinding>>();
        // Set create info
        let create_info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(&bindings);
        // Create descriptor set layout
        let layout = unsafe {
            context
                .try_borrow()?
                .logical_device()
                .create_descriptor_set_layout(&create_info, None)
        }?;
        // Return descriptor set layout
        Ok(Self {
            layout: VKHandle::new(context, layout, false),
            count,
            descriptors,
        })
    }
}

impl VKObject<vk::DescriptorSetLayout> for DescriptorSetLayout {
    fn handle(&self) -> &VKHandle<vk::DescriptorSetLayout> {
        &self.layout
    }

    fn handle_mut(&mut self) -> &mut VKHandle<vk::DescriptorSetLayout> {
        &mut self.layout
    }

    fn object_type() -> vk::DebugReportObjectTypeEXT {
        vk::DebugReportObjectTypeEXT::DESCRIPTOR_SET_LAYOUT
    }

    fn set_children_names(&mut self) -> Result<(), FennecError> {
        Ok(())
    }
}

/// Describes a descriptor in a descriptor set layout
#[derive(Default, Clone)]
pub struct Descriptor {
    /// Which shader stage to bind the descriptor in
    pub shader_stage: vk::ShaderStageFlags,
    /// Which location in the bound pipeline's shader stages to bind the descriptor to
    pub shader_binding_location: u32,
    /// The type of descriptor to allocate
    pub descriptor_type: vk::DescriptorType,
    /// The number of elements in this descriptor (>1 makes it an array)
    pub count: u32,
}
