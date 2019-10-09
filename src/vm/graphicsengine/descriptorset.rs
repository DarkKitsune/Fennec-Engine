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
}

impl DescriptorPool {
    /// DescriptorPool factory method
    pub fn new(
        context: &Rc<RefCell<Context>>,
        set_allocations: &[DescriptorSetAllocation],
        advanced_settings: Option<AdvancedDescriptorPoolSettings>,
    ) -> Result<Self, FennecError> {
        let advanced_settings = advanced_settings.unwrap_or_default();
        // Set create info
        let pool_sizes = set_allocations
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
            .max_sets(set_allocations.iter().map(|alloc| alloc.count).sum())
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
        })
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
        Ok(())
    }
}

/// Advanced settings to be used in descriptor pool factory methods
#[derive(Default, Copy, Clone)]
pub struct AdvancedDescriptorPoolSettings {
    /// Allow use of DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL_EXT *(default=false)*
    pub update_after_bind: Option<bool>,
}

/// Describes the required allocation for a type of descriptor set from a descriptor pool
#[derive(Default, Clone)]
pub struct DescriptorSetAllocation {
    /// Number of this type of descriptor set to allocate resources for
    pub count: u32,
    /// The list of descriptors in the descriptor set
    pub descriptors: Vec<Descriptor>,
}

/// Describes the required allocation for a type of descriptor set from a descriptor pool
#[derive(Default, Copy, Clone)]
pub struct Descriptor {
    /// The number of this descriptor to allocate
    pub count: u32,
    /// The type of descriptor to allocate
    pub descriptor_type: vk::DescriptorType,
}
