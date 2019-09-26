use super::memory::Memory;
use super::vkobject::{VKHandle, VKObject};
use super::Context;
use crate::error::FennecError;
use ash::version::DeviceV1_0;
use ash::vk;
use std::cell::RefCell;
use std::rc::Rc;
pub use vk::{
    Extent2D, Format, ImageCreateFlags, ImageLayout, ImageTiling, ImageUsageFlags, SampleCountFlags,
};

/// A 2-dimensional image
pub struct Image2D {
    image: VKHandle<vk::Image>,
    memory: Memory,
}

impl Image2D {
    /// Image2D factory method\
    /// ``context``: Graphics context\
    /// ``extent``: The dimensions of the image\
    /// ``usage``: How the image will be used\
    /// ``flags``: Image creation flags *(default=Default)*\
    // TODO: ^ clarify this ^
    /// ``format``: The pixel format of the image *(default=B8G8R8A8_UNORM)*\
    /// ``simultaneous_use``: Whether the image can be used by multiple queue families? *(default=false)*\
    // TODO: ^ clarify this ^
    /// ``initial_layout``: Initial layout of the image after creation *(default=GENERAL)*\
    /// ``mip_levels``: Number of mipmap levels *(default=1)*\
    /// ``layers``: Number of array layers *(default=1)*
    pub fn new_2d(
        context: &Rc<RefCell<Context>>,
        extent: Extent2D,
        usage: ImageUsageFlags,
        flags: Option<ImageCreateFlags>,
        format: Option<Format>,
        simultaneous_use: Option<bool>,
        initial_layout: Option<ImageLayout>,
        mip_levels: Option<u32>,
        layers: Option<u32>,
        sample_count: Option<SampleCountFlags>,
        image_tiling: Option<ImageTiling>,
    ) -> Result<Self, FennecError> {
        // Check that mip_levels is above 0 and below u32::max / 2
        if let Some(mip_levels) = mip_levels {
            if mip_levels == 0 {
                return Err(FennecError::new(
                    "# of mipmap levels must be greater than 0",
                ));
            }
            if mip_levels > std::u32::MAX / 2 {
                return Err(FennecError::new(format!(
                    "# of mipmap levels is extremely high ({}); possible underflow",
                    mip_levels
                )));
            }
        }
        // Check that layers is above 0 and below u32::max / 2
        if let Some(layers) = layers {
            if layers == 0 {
                return Err(FennecError::new("# of layers must be greater than 0"));
            }
            if layers > std::u32::MAX / 2 {
                return Err(FennecError::new(format!(
                    "# of layers is extremely high ({}); possible underflow",
                    layers
                )));
            }
        }
        // Set image create info
        let create_info = vk::ImageCreateInfo::builder()
            .flags(flags.unwrap_or_default())
            .image_type(vk::ImageType::TYPE_2D)
            .format(format.unwrap_or(Format::B8G8R8A8_UNORM))
            .extent(vk::Extent3D {
                width: extent.width,
                height: extent.height,
                depth: 1,
            })
            .mip_levels(mip_levels.unwrap_or(1))
            .array_layers(layers.unwrap_or(1))
            .tiling(image_tiling.unwrap_or(ImageTiling::OPTIMAL))
            .samples(sample_count.unwrap_or(SampleCountFlags::TYPE_1))
            .usage(usage)
            .sharing_mode(if simultaneous_use.unwrap_or(false) {
                vk::SharingMode::CONCURRENT
            } else {
                vk::SharingMode::EXCLUSIVE
            })
            .initial_layout(initial_layout.unwrap_or(ImageLayout::GENERAL))
            .build();
        // Create image and memory
        let context_borrowed = context.try_borrow()?;
        let logical_device = context_borrowed.logical_device();
        let image = unsafe { logical_device.create_image(&create_info, None) }?;
        let memory = Memory::new(context, unsafe {
            logical_device.get_image_memory_requirements(image)
        })?;
        // Bind memory to image
        unsafe { logical_device.bind_image_memory(image, *memory.handle().handle(), 0) }?;
        // Return image
        Ok(Self {
            image: VKHandle::new(context, image, false),
            memory,
        })
    }

    /// Gets the backing memory of the image
    pub fn memory(&self) -> &Memory {
        &self.memory
    }
}

impl VKObject<vk::Image> for Image2D {
    fn handle(&self) -> &VKHandle<vk::Image> {
        &self.image
    }

    fn handle_mut(&mut self) -> &mut VKHandle<vk::Image> {
        &mut self.image
    }

    fn object_type() -> vk::DebugReportObjectTypeEXT {
        vk::DebugReportObjectTypeEXT::IMAGE
    }
}

impl Image for Image2D {
    fn image_handle(&self) -> &VKHandle<vk::Image> {
        self.handle()
    }

    fn memory(&self) -> Option<&Memory> {
        Some(self.memory())
    }
}

/// Trait for Vulkan images
pub trait Image {
    /// Gets the handle of the wrapped Vulkan image
    fn image_handle(&self) -> &VKHandle<vk::Image>;
    /// Gets the backing memory of the image
    fn memory(&self) -> Option<&Memory>;
}
