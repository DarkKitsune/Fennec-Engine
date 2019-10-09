use super::vkobject::{VKHandle, VKObject};
use super::Context;
use crate::error::FennecError;
use ash::version::DeviceV1_0;
use ash::vk;
use std::cell::RefCell;
use std::rc::Rc;

/// A Vulkan fence
pub struct Fence {
    fence: VKHandle<vk::Fence>,
}

impl Fence {
    /// Fence factory method
    pub fn new(context: &Rc<RefCell<Context>>, signaled: bool) -> Result<Self, FennecError> {
        let create_info = vk::FenceCreateInfo::builder().flags(if signaled {
            vk::FenceCreateFlags::SIGNALED
        } else {
            Default::default()
        });
        let fence = unsafe {
            context
                .try_borrow()?
                .logical_device()
                .create_fence(&create_info, None)
        }?;
        Ok(Self {
            fence: VKHandle::new(context, fence, false),
        })
    }

    /// Get the fence status
    pub fn status(&self) -> Result<FenceStatus, FennecError> {
        let status = unsafe {
            self.context()
                .try_borrow()?
                .logical_device()
                .get_fence_status(*self.handle().handle())
        };
        match status {
            Ok(_) => Ok(FenceStatus::Signaled),
            Err(result) => match result {
                vk::Result::SUCCESS => Ok(FenceStatus::Signaled),
                vk::Result::NOT_READY => Ok(FenceStatus::Unsignaled),
                _ => Err(FennecError::new(format!("Status was {:?}", result))),
            },
        }
    }

    /// Get whether the fence is signaled
    pub fn signaled(&self) -> Result<bool, FennecError> {
        match self.status()? {
            FenceStatus::Signaled => Ok(true),
            _ => Ok(false),
        }
    }

    /// Pause the current thread to wait on the fence
    pub fn wait(&mut self, timeout_nanoseconds: Option<u64>) -> Result<(), FennecError> {
        Ok(unsafe {
            self.context()
                .try_borrow()?
                .logical_device()
                .wait_for_fences(
                    &[*self.handle().handle()],
                    false,
                    timeout_nanoseconds.unwrap_or(std::u64::MAX),
                )
        }?)
    }

    /// Reset the fence status to unsignaled
    pub fn reset(&mut self) -> Result<(), FennecError> {
        Ok(unsafe {
            self.context()
                .try_borrow()?
                .logical_device()
                .reset_fences(&[*self.handle().handle()])
        }?)
    }
}

impl VKObject<vk::Fence> for Fence {
    fn handle(&self) -> &VKHandle<vk::Fence> {
        &self.fence
    }

    fn handle_mut(&mut self) -> &mut VKHandle<vk::Fence> {
        &mut self.fence
    }

    fn object_type() -> vk::DebugReportObjectTypeEXT {
        vk::DebugReportObjectTypeEXT::FENCE
    }

    fn set_children_names(&mut self) -> Result<(), FennecError> {
        Ok(())
    }
}

/// A status of a fence
#[derive(Copy, Clone)]
pub enum FenceStatus {
    Signaled,
    Unsignaled,
}

/// A Vulkan semaphore
pub struct Semaphore {
    semaphore: VKHandle<vk::Semaphore>,
}

impl Semaphore {
    /// Semaphore factory method
    pub fn new(context: &Rc<RefCell<Context>>) -> Result<Self, FennecError> {
        let create_info = vk::SemaphoreCreateInfo::builder();
        let semaphore = unsafe {
            context
                .try_borrow()?
                .logical_device()
                .create_semaphore(&create_info, None)
        }?;
        Ok(Self {
            semaphore: VKHandle::new(context, semaphore, false),
        })
    }
}

impl VKObject<vk::Semaphore> for Semaphore {
    fn handle(&self) -> &VKHandle<vk::Semaphore> {
        &self.semaphore
    }

    fn handle_mut(&mut self) -> &mut VKHandle<vk::Semaphore> {
        &mut self.semaphore
    }

    fn object_type() -> vk::DebugReportObjectTypeEXT {
        vk::DebugReportObjectTypeEXT::SEMAPHORE
    }

    fn set_children_names(&mut self) -> Result<(), FennecError> {
        Ok(())
    }
}
