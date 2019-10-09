use super::image::Image;
use super::vkobject::{VKHandle, VKObject};
use super::Context;
use crate::error::FennecError;
use ash::version::DeviceV1_0;
use ash::vk;
use std::cell::RefCell;
use std::rc::Rc;

/// An image view
pub struct ImageView {
    image_view: VKHandle<vk::ImageView>,
    extent: vk::Extent3D,
}

impl ImageView {
    /// ImageView factory method
    pub fn new(
        context: &Rc<RefCell<Context>>,
        image: &impl Image,
        range: &vk::ImageSubresourceRange,
        components: Option<vk::ComponentMapping>,
    ) -> Result<Self, FennecError> {
        // Set image view create info
        let create_info = vk::ImageViewCreateInfo::builder()
            .image(*image.image_handle().handle())
            .format(image.format())
            .subresource_range(*range)
            .view_type(image.image_view_type())
            .components(components.unwrap_or_default());
        // Create image view
        let image_view = unsafe {
            context
                .try_borrow()?
                .logical_device()
                .create_image_view(&create_info, None)
        }?;
        // Return image view
        Ok(Self {
            image_view: VKHandle::new(context, image_view, false),
            extent: image.extent(),
        })
    }

    pub fn extent(&self) -> vk::Extent3D {
        self.extent
    }
}

impl VKObject<vk::ImageView> for ImageView {
    fn handle(&self) -> &VKHandle<vk::ImageView> {
        &self.image_view
    }

    fn handle_mut(&mut self) -> &mut VKHandle<vk::ImageView> {
        &mut self.image_view
    }

    fn object_type() -> vk::DebugReportObjectTypeEXT {
        vk::DebugReportObjectTypeEXT::IMAGE_VIEW
    }

    fn set_children_names(&mut self) -> Result<(), FennecError> {
        Ok(())
    }
}
