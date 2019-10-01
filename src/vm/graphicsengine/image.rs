use super::imageview::ImageView;
use super::memory::Memory;
use super::vkobject::{VKHandle, VKObject};
use super::Context;
use crate::error::FennecError;
use ash::version::DeviceV1_0;
use ash::vk;
use std::cell::RefCell;
use std::rc::Rc;

/// A 2-dimensional image
pub struct Image2D {
    image: VKHandle<vk::Image>,
    memory: Memory,
    format: vk::Format,
    extent: vk::Extent2D,
}

impl Image2D {
    /// Image2D factory method\
    /// ``extent``: The dimensions of the image\
    /// ``usage``: How the image will be used\
    /// ``format``: The pixel format of the image *(default=B8G8R8A8_UNORM)*\
    /// ``initial_layout``: Initial layout of the image after creation *(default=GENERAL)*\
    /// ``advanced_settings``: Advanced creation settings
    pub fn new(
        context: &Rc<RefCell<Context>>,
        extent: vk::Extent2D,
        usage: vk::ImageUsageFlags,
        format: Option<vk::Format>,
        initial_layout: Option<vk::ImageLayout>,
        advanced_settings: Option<AdvancedImageSettings>,
    ) -> Result<Self, FennecError> {
        let format = format.unwrap_or(vk::Format::B8G8R8A8_UNORM);
        let advanced_settings = advanced_settings.unwrap_or_default();
        // Check that mip_levels is greater than 0 and below u32::MAX / 2
        if let Some(mip_levels) = advanced_settings.mip_levels {
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
        // Check that layers is greater than 0 and below u32::MAX / 2
        if let Some(layers) = advanced_settings.layers {
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
        // Check that extent.width is greater than 0 and below u32::MAX / 2
        if extent.width == 0 {
            return Err(FennecError::new("extent.width must be greater than 0"));
        }
        if extent.width > std::u32::MAX / 2 {
            return Err(FennecError::new(format!(
                "extent.width is extremely high ({}); possible underflow",
                extent.width
            )));
        }
        // Check that extent.height is greater than 0 and below u32::MAX / 2
        if extent.height == 0 {
            return Err(FennecError::new("extent.height must be greater than 0"));
        }
        if extent.height > std::u32::MAX / 2 {
            return Err(FennecError::new(format!(
                "extent.height is extremely high ({}); possible underflow",
                extent.height
            )));
        }
        // Set image create info
        let create_info = vk::ImageCreateInfo::builder()
            .flags(advanced_settings.flags.unwrap_or_default())
            .image_type(vk::ImageType::TYPE_2D)
            .format(format)
            .extent(vk::Extent3D {
                width: extent.width,
                height: extent.height,
                depth: 1,
            })
            .mip_levels(advanced_settings.mip_levels.unwrap_or(1))
            .array_layers(advanced_settings.layers.unwrap_or(1))
            .tiling(
                advanced_settings
                    .image_tiling
                    .unwrap_or(vk::ImageTiling::OPTIMAL),
            )
            .samples(
                advanced_settings
                    .sample_count
                    .unwrap_or(vk::SampleCountFlags::TYPE_1),
            )
            .usage(usage)
            .sharing_mode(if advanced_settings.simultaneous_use.unwrap_or(false) {
                vk::SharingMode::CONCURRENT
            } else {
                vk::SharingMode::EXCLUSIVE
            })
            .initial_layout(initial_layout.unwrap_or(vk::ImageLayout::GENERAL))
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
            format,
            extent,
        })
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
        Some(&self.memory)
    }

    fn format(&self) -> vk::Format {
        self.format
    }

    fn image_view_type(&self) -> vk::ImageViewType {
        vk::ImageViewType::TYPE_2D
    }

    fn extent(&self) -> vk::Extent3D {
        vk::Extent3D {
            width: self.extent.width,
            height: self.extent.height,
            depth: 1,
        }
    }

    fn view(
        &self,
        range: &vk::ImageSubresourceRange,
        components: Option<vk::ComponentMapping>,
    ) -> Result<ImageView, FennecError> {
        let mut view = ImageView::new(self.image_handle().context(), self, range, components)?;
        view.set_name(&format!("view into {}", self.name()))?;
        Ok(view)
    }
}

/// Advanced settings to be used in image factory methods
#[derive(Default, Copy, Clone)]
pub struct AdvancedImageSettings {
    // TODO: v clarify this v
    /// Image creation flags *(default=Default)*
    pub flags: Option<vk::ImageCreateFlags>,
    // TODO: v clarify this v
    /// Whether the image can be used by multiple queue families? *(default=false)*
    pub simultaneous_use: Option<bool>,
    /// Number of mipmap levels *(default=1)*
    pub mip_levels: Option<u32>,
    /// Number of array layers *(default=1)*
    pub layers: Option<u32>,
    /// Number of samples per pixel *(default=TYPE_1)*
    pub sample_count: Option<vk::SampleCountFlags>,
    /// Tiling arrangement for image data *(default=OPTIMAL)*
    pub image_tiling: Option<vk::ImageTiling>,
}

/// Trait for Vulkan images
pub trait Image {
    /// Gets the handle of the wrapped Vulkan image
    fn image_handle(&self) -> &VKHandle<vk::Image>;
    /// Gets the backing memory of the image
    fn memory(&self) -> Option<&Memory>;
    /// Gets the pixel format of the image
    fn format(&self) -> vk::Format;
    /// Get the correct type for a view of the image
    fn image_view_type(&self) -> vk::ImageViewType;
    /// Get the extent of the image
    fn extent(&self) -> vk::Extent3D;
    /// Create an ImageView of the image
    fn view(
        &self,
        range: &vk::ImageSubresourceRange,
        components: Option<vk::ComponentMapping>,
    ) -> Result<ImageView, FennecError>;

    /// Create a subresource range
    fn range(
        &self,
        aspects: vk::ImageAspectFlags,
        base_layer: u32,
        layer_count: u32,
        base_mip: u32,
        mip_count: u32,
    ) -> vk::ImageSubresourceRange {
        vk::ImageSubresourceRange::builder()
            .aspect_mask(aspects)
            .base_array_layer(base_layer)
            .layer_count(layer_count)
            .base_mip_level(base_mip)
            .level_count(mip_count)
            .build()
    }

    /// Create a subresource range pointing to the color aspect of layer 0, mipmap level 0
    fn range_color_basic(&self) -> vk::ImageSubresourceRange {
        vk::ImageSubresourceRange::builder()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .base_array_layer(0)
            .layer_count(1)
            .base_mip_level(0)
            .level_count(1)
            .build()
    }

    /// Create a subresource range pointing to the depth & stencil aspects of layer 0, mipmap level 0
    fn range_depth_stencil_basic(&self) -> vk::ImageSubresourceRange {
        vk::ImageSubresourceRange::builder()
            .aspect_mask(vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL)
            .base_array_layer(0)
            .layer_count(1)
            .base_mip_level(0)
            .level_count(1)
            .build()
    }
}
