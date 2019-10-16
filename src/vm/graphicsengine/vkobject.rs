use super::Context;
use crate::error::FennecError;
use ash::version::DeviceV1_0;
use ash::vk;
use std::cell::RefCell;
use std::ffi::CString;
use std::rc::Rc;

/// Trait for valid handle types
pub trait HandleType {
    /// Destroy the object pointed to by the handle, if possible
    fn destroy(&mut self, context: &Rc<RefCell<Context>>) -> Result<(), FennecError>;
}

impl HandleType for vk::Fence {
    fn destroy(&mut self, context: &Rc<RefCell<Context>>) -> Result<(), FennecError> {
        unsafe {
            context
                .try_borrow()?
                .logical_device()
                .destroy_fence(*self, None)
        };
        Ok(())
    }
}

impl HandleType for vk::Semaphore {
    fn destroy(&mut self, context: &Rc<RefCell<Context>>) -> Result<(), FennecError> {
        unsafe {
            context
                .try_borrow()?
                .logical_device()
                .destroy_semaphore(*self, None)
        };
        Ok(())
    }
}

impl HandleType for vk::Queue {
    fn destroy(&mut self, _context: &Rc<RefCell<Context>>) -> Result<(), FennecError> {
        Ok(())
    }
}

impl HandleType for vk::CommandPool {
    fn destroy(&mut self, context: &Rc<RefCell<Context>>) -> Result<(), FennecError> {
        unsafe {
            context
                .try_borrow()?
                .logical_device()
                .destroy_command_pool(*self, None);
        };
        Ok(())
    }
}

impl HandleType for vk::CommandBuffer {
    fn destroy(&mut self, _context: &Rc<RefCell<Context>>) -> Result<(), FennecError> {
        Ok(())
    }
}

impl HandleType for vk::SwapchainKHR {
    fn destroy(&mut self, context: &Rc<RefCell<Context>>) -> Result<(), FennecError> {
        unsafe {
            context
                .try_borrow()?
                .functions()
                .device_extensions()
                .swapchain()
                .destroy_swapchain(*self, None)
        };
        Ok(())
    }
}

impl HandleType for vk::Image {
    fn destroy(&mut self, context: &Rc<RefCell<Context>>) -> Result<(), FennecError> {
        unsafe {
            context
                .try_borrow()?
                .logical_device()
                .destroy_image(*self, None)
        };
        Ok(())
    }
}

impl HandleType for vk::DeviceMemory {
    fn destroy(&mut self, context: &Rc<RefCell<Context>>) -> Result<(), FennecError> {
        unsafe {
            context
                .try_borrow()?
                .logical_device()
                .free_memory(*self, None)
        };
        Ok(())
    }
}

impl HandleType for vk::Pipeline {
    fn destroy(&mut self, context: &Rc<RefCell<Context>>) -> Result<(), FennecError> {
        unsafe {
            context
                .try_borrow()?
                .logical_device()
                .destroy_pipeline(*self, None)
        };
        Ok(())
    }
}

impl HandleType for vk::PipelineLayout {
    fn destroy(&mut self, context: &Rc<RefCell<Context>>) -> Result<(), FennecError> {
        unsafe {
            context
                .try_borrow()?
                .logical_device()
                .destroy_pipeline_layout(*self, None)
        };
        Ok(())
    }
}

impl HandleType for vk::RenderPass {
    fn destroy(&mut self, context: &Rc<RefCell<Context>>) -> Result<(), FennecError> {
        unsafe {
            context
                .try_borrow()?
                .logical_device()
                .destroy_render_pass(*self, None)
        };
        Ok(())
    }
}

impl HandleType for vk::Framebuffer {
    fn destroy(&mut self, context: &Rc<RefCell<Context>>) -> Result<(), FennecError> {
        unsafe {
            context
                .try_borrow()?
                .logical_device()
                .destroy_framebuffer(*self, None)
        };
        Ok(())
    }
}

impl HandleType for vk::ImageView {
    fn destroy(&mut self, context: &Rc<RefCell<Context>>) -> Result<(), FennecError> {
        unsafe {
            context
                .try_borrow()?
                .logical_device()
                .destroy_image_view(*self, None)
        };
        Ok(())
    }
}

impl HandleType for vk::DescriptorPool {
    fn destroy(&mut self, context: &Rc<RefCell<Context>>) -> Result<(), FennecError> {
        unsafe {
            context
                .try_borrow()?
                .logical_device()
                .destroy_descriptor_pool(*self, None)
        };
        Ok(())
    }
}

impl HandleType for vk::Buffer {
    fn destroy(&mut self, context: &Rc<RefCell<Context>>) -> Result<(), FennecError> {
        unsafe {
            context
                .try_borrow()?
                .logical_device()
                .destroy_buffer(*self, None)
        };
        Ok(())
    }
}

impl HandleType for vk::ShaderModule {
    fn destroy(&mut self, context: &Rc<RefCell<Context>>) -> Result<(), FennecError> {
        unsafe {
            context
                .try_borrow()?
                .logical_device()
                .destroy_shader_module(*self, None)
        };
        Ok(())
    }
}

impl HandleType for vk::DescriptorSet {
    fn destroy(&mut self, _context: &Rc<RefCell<Context>>) -> Result<(), FennecError> {
        Ok(())
    }
}

impl HandleType for vk::DescriptorSetLayout {
    fn destroy(&mut self, context: &Rc<RefCell<Context>>) -> Result<(), FennecError> {
        unsafe {
            context
                .try_borrow()?
                .logical_device()
                .destroy_descriptor_set_layout(*self, None)
        };
        Ok(())
    }
}

pub struct VKHandle<THandleType>
where
    THandleType: HandleType + Copy + vk::Handle,
{
    context: Rc<RefCell<Context>>,
    handle: THandleType,
    protected: bool,
    name: String,
}

/// A wrapper around a raw Vulkan handle
impl<THandleType> VKHandle<THandleType>
where
    THandleType: HandleType + Copy + vk::Handle,
{
    /// VKHandle factory method
    pub fn new(context: &Rc<RefCell<Context>>, handle: THandleType, protected: bool) -> Self {
        Self {
            context: context.clone(),
            handle,
            protected,
            name: String::from("Unnamed"),
        }
    }

    /// Get the graphics context the handle is associated with
    pub fn context(&self) -> &Rc<RefCell<Context>> {
        &self.context
    }

    /// Get the graphics context the handle is associated with
    pub fn context_mut(&mut self) -> &mut Rc<RefCell<Context>> {
        &mut self.context
    }

    /// Get the raw handle wrapped by the VKHandle
    pub fn handle(&self) -> &THandleType {
        &self.handle
    }

    /// Get the raw handle wrapped by the VKHandle
    pub fn handle_mut(&mut self) -> &mut THandleType {
        &mut self.handle
    }

    /// Set the name of the VKHandle (usually shouldn't be used directly)
    pub fn set_name(&mut self, name: &str) {
        self.name = String::from(name);
    }

    /// Get the name of the VKHandle
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl<THandleType> Drop for VKHandle<THandleType>
where
    THandleType: HandleType + Copy + vk::Handle,
{
    fn drop(&mut self) {
        // Don't do anything if self.protected == true
        if self.protected {
            return;
        }
        // Log that we are dropping this
        println!("Dropping {}", self.name());
        // Destroy the object pointed to by the handle
        let mut handle = *self.handle_mut();
        handle
            .destroy(self.context())
            .expect("Error occured when dropping VKHandle");
    }
}

pub trait VKObject<THandleType>
where
    THandleType: HandleType + Copy + vk::Handle + 'static,
{
    /// The VKHandle wrapping the raw Vulkan object handle
    fn handle(&self) -> &VKHandle<THandleType>;
    /// The VKHandle wrapping the raw Vulkan object handle
    fn handle_mut(&mut self) -> &mut VKHandle<THandleType>;
    /// Get the type of the Vulkan object
    fn object_type() -> vk::DebugReportObjectTypeEXT;
    /// Update the name of children (should not normally be used)
    fn set_children_names(&mut self) -> Result<(), FennecError>;
    /// Set the name of the Vulkan object for debug info
    fn set_name(&mut self, name: &str) -> Result<(), FennecError> {
        // Set the name on the program side by setting the VKHandle's name
        self.handle_mut().set_name(name);
        // Set the name on the Vulkan side
        {
            let context = self.context().try_borrow()?;
            let cstr = CString::new(name).map_err(|err| {
                FennecError::from_error("Could not convert object name to a CString", Box::new(err))
            })?;
            let object_name = vk::DebugMarkerObjectNameInfoEXT::builder()
                .object(self.handle().handle().as_raw())
                .object_type(Self::object_type())
                .object_name(&cstr);
            unsafe {
                context
                    .functions()
                    .device_extensions()
                    .debug_marker()
                    .debug_marker_set_object_name(
                        context.logical_device().handle(),
                        &object_name,
                    )?;
            }
        }
        // Set name of children
        self.set_children_names()?;
        Ok(())
    }

    /// Get the name of the Vulkan object
    fn name(&self) -> &str {
        self.handle().name()
    }

    /// Get the associated graphics context
    fn context(&self) -> &Rc<RefCell<Context>> {
        self.handle().context()
    }

    /// Get the associated graphics context
    fn context_mut(&mut self) -> &mut Rc<RefCell<Context>> {
        self.handle_mut().context_mut()
    }

    fn with_name(mut self, name: &str) -> Result<Self, FennecError>
    where
        Self: Sized,
    {
        self.set_name(name)?;
        Ok(self)
    }
}
