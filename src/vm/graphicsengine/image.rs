use super::buffer::Buffer;
use super::imageview::ImageView;
use super::memory::Memory;
use super::queuefamily::{QueueFamily, QueueFamilyCollection};
use super::vkobject::{VKHandle, VKObject};
use super::Context;
use crate::error::FennecError;
use ash::version::DeviceV1_0;
use ash::vk;
use image::DynamicImage;
use std::cell::RefCell;
use std::rc::Rc;

/// The default image format
pub const DEFAULT_FORMAT: vk::Format = vk::Format::B8G8R8A8_UNORM;

/// A 2-dimensional image
pub struct Image2D {
    image: VKHandle<vk::Image>,
    memory: Memory,
    format: vk::Format,
    extent: vk::Extent2D,
    mip_count: u32,
}

impl Image2D {
    /// Image2D factory method\
    /// ``extent``: The dimensions of the image\
    /// ``usage``: How the image will be used\
    /// ``format``: The pixel format of the image *(default=B8G8R8A8_UNORM)*\
    /// ``initial_layout``: Initial layout of the image after creation *(default=UNDEFINED)*\
    /// ``advanced_settings``: Advanced creation settings
    pub fn new(
        context: &Rc<RefCell<Context>>,
        extent: vk::Extent2D,
        usage: vk::ImageUsageFlags,
        shared_among: &[&QueueFamily],
        format: Option<vk::Format>,
        initial_layout: Option<vk::ImageLayout>,
        advanced_settings: Option<AdvancedImageSettings>,
    ) -> Result<Self, FennecError> {
        let format = format.unwrap_or(DEFAULT_FORMAT);
        let advanced_settings = advanced_settings.unwrap_or_default();
        let shared_among = shared_among
            .iter()
            .map(|queue_family| queue_family.index())
            .collect::<Vec<u32>>();
        // Check that mip_levels is greater than 0
        if let Some(mip_levels) = advanced_settings.mip_count {
            if mip_levels == 0 {
                return Err(FennecError::new(
                    "# of mipmap levels must be greater than 0",
                ));
            }
        }
        // Check that extent.width is greater than 0
        if extent.width == 0 {
            return Err(FennecError::new("extent.width must be greater than 0"));
        }
        // Check that extent.height is greater than 0
        if extent.height == 0 {
            return Err(FennecError::new("extent.height must be greater than 0"));
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
            .mip_levels(advanced_settings.mip_count.unwrap_or(1))
            .array_layers(1)
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
            .queue_family_indices(&shared_among)
            .initial_layout(initial_layout.unwrap_or(vk::ImageLayout::UNDEFINED));
        // Create image and memory
        let context_borrowed = context.try_borrow()?;
        let logical_device = context_borrowed.logical_device();
        let image = unsafe { logical_device.create_image(&create_info, None) }?;
        let memory = Memory::new(
            context,
            unsafe { logical_device.get_image_memory_requirements(image) },
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;
        // Bind memory to image
        unsafe { logical_device.bind_image_memory(image, memory.handle(), 0) }?;
        // Return image
        Ok(Self {
            image: VKHandle::new(context, image, false),
            memory,
            format,
            extent,
            mip_count: advanced_settings.mip_count.unwrap_or(1),
        })
    }
}

impl VKObject<vk::Image> for Image2D {
    fn wrapped_handle(&self) -> &VKHandle<vk::Image> {
        &self.image
    }

    fn wrapped_handle_mut(&mut self) -> &mut VKHandle<vk::Image> {
        &mut self.image
    }

    fn object_type() -> vk::DebugReportObjectTypeEXT {
        vk::DebugReportObjectTypeEXT::IMAGE
    }

    fn set_children_names(&mut self) -> Result<(), FennecError> {
        self.memory.set_name(&format!("{}.memory", self.name()))?;
        Ok(())
    }
}

impl Image for Image2D {
    fn image_handle(&self) -> &VKHandle<vk::Image> {
        self.wrapped_handle()
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

    fn layer_count(&self) -> u32 {
        1
    }

    fn mip_count(&self) -> u32 {
        self.mip_count
    }

    fn view(
        &self,
        range: &vk::ImageSubresourceRange,
        components: Option<vk::ComponentMapping>,
    ) -> Result<ImageView, FennecError> {
        let view = ImageView::new(self.image_handle().context(), self, range, components)?
            .with_name(&format!("view into {}", self.name()))?;
        Ok(view)
    }
}

/// Advanced settings to be used in image factory methods
#[derive(Default)]
pub struct AdvancedImageSettings {
    /// Image creation flags *(default=Default)*
    pub flags: Option<vk::ImageCreateFlags>,
    /// Whether the image can be used by multiple queue families concurrently *(default=false)*
    pub simultaneous_use: Option<bool>,
    /// Number of mipmap levels *(default=1)*
    pub mip_count: Option<u32>,
    /// Number of samples per pixel *(default=TYPE_1)*
    pub sample_count: Option<vk::SampleCountFlags>,
    /// Tiling arrangement for image data *(default=OPTIMAL)*
    pub image_tiling: Option<vk::ImageTiling>,
}

/// Trait for Vulkan images
pub trait Image: VKObject<vk::Image> + Sized {
    /// Gets the handle of the wrapped Vulkan image
    fn image_handle(&self) -> &VKHandle<vk::Image>;
    /// Gets the backing memory of the image
    fn memory(&self) -> Option<&Memory>;
    /// Gets the pixel format of the image
    fn format(&self) -> vk::Format;
    /// Gets the correct type for a view of the image
    fn image_view_type(&self) -> vk::ImageViewType;
    /// Gets the extent of the image
    fn extent(&self) -> vk::Extent3D;
    /// Gets the number of layers of the image
    fn layer_count(&self) -> u32;
    /// Gets the number of mip levels of the image
    fn mip_count(&self) -> u32;
    /// Creates an ImageView of the image
    fn view(
        &self,
        range: &vk::ImageSubresourceRange,
        components: Option<vk::ComponentMapping>,
    ) -> Result<ImageView, FennecError>;

    /// Verifies that a given region falls within the image's bounds
    fn verify_region_is_inside(
        &self,
        offset: vk::Offset3D,
        extent: vk::Extent3D,
    ) -> Result<(), FennecError> {
        let region_mx = offset.x;
        let region_px = region_mx + extent.width as i32;
        let region_my = offset.y;
        let region_py = region_my + extent.height as i32;
        let region_mz = offset.z;
        let region_pz = region_mz + extent.depth as i32;
        if region_mx < 0 {
            return Err(FennecError::new(&format!(
                "-X edge of region in image ({}) is {} which falls outside of the image",
                self.name(),
                region_mx
            )));
        }
        if region_px > self.extent().width as i32 {
            return Err(FennecError::new(&format!(
                "+X edge of region in image ({}) is {} which falls outside of the image",
                self.name(),
                region_px
            )));
        }
        if region_my < 0 {
            return Err(FennecError::new(&format!(
                "-Y edge of region in image ({}) is {} which falls outside of the image",
                self.name(),
                region_my
            )));
        }
        if region_py > self.extent().height as i32 {
            return Err(FennecError::new(&format!(
                "+Y edge of region in image ({}) is {} which falls outside of the image",
                self.name(),
                region_py
            )));
        }
        if region_mz < 0 {
            return Err(FennecError::new(&format!(
                "-Z edge of region in image ({}) is {} which falls outside of the image",
                self.name(),
                region_mz
            )));
        }
        if region_pz > self.extent().depth as i32 {
            return Err(FennecError::new(&format!(
                "+Z edge of region in image ({}) is {} which falls outside of the image",
                self.name(),
                region_pz
            )));
        }
        Ok(())
    }

    /// Create a subresource range
    fn range(
        &self,
        aspects: vk::ImageAspectFlags,
        base_layer: u32,
        layer_count: u32,
        base_mip: u32,
        mip_count: u32,
    ) -> vk::ImageSubresourceRange {
        *vk::ImageSubresourceRange::builder()
            .aspect_mask(aspects)
            .base_array_layer(base_layer)
            .layer_count(layer_count)
            .base_mip_level(base_mip)
            .level_count(mip_count)
    }

    /// Create a subresource range pointing to the color aspect
    /// of layer 0, mipmap level 0
    fn range_color_basic(&self) -> vk::ImageSubresourceRange {
        *vk::ImageSubresourceRange::builder()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .base_array_layer(0)
            .layer_count(1)
            .base_mip_level(0)
            .level_count(1)
    }

    /// Create a subresource range pointing to the depth & stencil
    /// aspects of layer 0, mipmap level 0
    fn range_depth_stencil_basic(&self) -> vk::ImageSubresourceRange {
        *vk::ImageSubresourceRange::builder()
            .aspect_mask(vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL)
            .base_array_layer(0)
            .layer_count(1)
            .base_mip_level(0)
            .level_count(1)
    }

    /// Create a subresource layers description
    fn layers(
        &self,
        aspects: vk::ImageAspectFlags,
        base_layer: u32,
        layer_count: u32,
        mip_level: u32,
    ) -> vk::ImageSubresourceLayers {
        *vk::ImageSubresourceLayers::builder()
            .aspect_mask(aspects)
            .base_array_layer(base_layer)
            .layer_count(layer_count)
            .mip_level(mip_level)
    }

    /// Create a subresource layers description pointing to the
    /// color aspect of layer 0, mipmap level 0
    fn layers_color_basic(&self) -> vk::ImageSubresourceLayers {
        *vk::ImageSubresourceLayers::builder()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .base_array_layer(0)
            .layer_count(1)
            .mip_level(0)
    }

    /// Create a subresource layers description pointing to the
    /// depth & stencil aspects of layer 0, mipmap level 0
    fn layers_depth_stencil_basic(&self) -> vk::ImageSubresourceLayers {
        *vk::ImageSubresourceLayers::builder()
            .aspect_mask(vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL)
            .base_array_layer(0)
            .layer_count(1)
            .mip_level(0)
    }

    /// Convert a pixel position to texture coordinates
    fn texture_coordinates(&self, pixel: vk::Offset3D) -> (f32, f32, f32) {
        (
            pixel.x as f32 / self.extent().width as f32,
            pixel.y as f32 / self.extent().height as f32,
            pixel.x as f32 / self.extent().depth as f32,
        )
    }

    /// Load compressed image data into the image
    fn load_compressed_image(
        &self,
        queue_family_collection: &mut QueueFamilyCollection,
        source: &DynamicImage,
        consuming_stage: vk::PipelineStageFlags,
        new_layout: vk::ImageLayout,
        new_access: vk::AccessFlags,
    ) -> Result<(), FennecError> {
        // Create and fill staging buffer
        let staging_buffer = {
            let texture_source_raw = source.to_bgra().into_raw();
            unsafe {
                Buffer::from_bytes(
                    self.context(),
                    &texture_source_raw,
                    texture_source_raw.len(),
                    vk::BufferUsageFlags::TRANSFER_SRC,
                    None,
                    None,
                )
            }?
            .with_name(&format!(
                "Image::load_compressed_image::staging_buffer({})",
                self.name()
            ))?
        };
        // Write command buffer to copy buffer to image
        let copy_command_buffers_handle = {
            let (copy_command_buffers_handle, copy_command_buffers) = queue_family_collection
                .graphics_mut()
                .command_pools_mut()
                .unwrap()
                .transient_mut()
                .create_command_buffers(1)?;
            let writer = copy_command_buffers[0].begin(true, false)?;
            writer.pipeline_barrier(
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::TRANSFER,
                None,
                None,
                None,
                Some(&[*vk::ImageMemoryBarrier::builder()
                    .image(self.handle())
                    .subresource_range(self.range_color_basic())
                    .old_layout(vk::ImageLayout::UNDEFINED)
                    .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                    .src_access_mask(Default::default())
                    .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE)]),
            )?;
            unsafe {
                writer.copy_buffer_to_image(
                    &staging_buffer,
                    self,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    &[Buffer::copy_to_image(
                        0,
                        self,
                        vk::ImageAspectFlags::COLOR,
                        0,
                    )],
                )?;
            }
            writer.pipeline_barrier(
                vk::PipelineStageFlags::TRANSFER,
                consuming_stage,
                None,
                None,
                None,
                Some(&[*vk::ImageMemoryBarrier::builder()
                    .image(self.handle())
                    .subresource_range(self.range_color_basic())
                    .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                    .new_layout(new_layout)
                    .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                    .dst_access_mask(new_access)]),
            )?;
            copy_command_buffers_handle
        };
        // Submit command buffer
        let queue = queue_family_collection
            .graphics()
            .queue_of_priority(1.0)
            .unwrap();
        queue.submit(
            Some(&[&queue_family_collection
                .graphics()
                .command_pools()
                .unwrap()
                .transient()
                .command_buffers(copy_command_buffers_handle)?[0]]),
            None,
            None,
            None,
        )?;
        // Wait for the copy to be finished
        queue.wait()?;
        // Clean up command buffers
        queue_family_collection
            .graphics_mut()
            .command_pools_mut()
            .unwrap()
            .transient_mut()
            .destroy_command_buffers(copy_command_buffers_handle)?;
        Ok(())
    }
}
