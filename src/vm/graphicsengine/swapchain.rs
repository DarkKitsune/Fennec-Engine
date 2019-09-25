use super::image::Image;
use super::memory::Memory;
use super::queue::Queue;
use super::sync::{Fence, Semaphore};
use super::vkobject::{VKHandle, VKObject};
use super::Context;
use crate::error::FennecError;
use crate::iteratorext::IteratorResults;
use ash::vk;
use std::cell::RefCell;
use std::rc::Rc;

/// The preferred swapchain image
const PREFERRED_SURFACE_FORMAT: vk::Format = vk::Format::B8G8R8A8_UNORM;
const PREFERRED_COLOR_SPACE: vk::ColorSpaceKHR = vk::ColorSpaceKHR::SRGB_NONLINEAR;
const PREFERRED_PRESENT_MODE: vk::PresentModeKHR = vk::PresentModeKHR::MAILBOX;

/// A swapchain
pub struct Swapchain {
    swapchain: VKHandle<vk::SwapchainKHR>,
    swapchain_images: Vec<SwapchainImage>,
}

impl Swapchain {
    /// Swapchain factory method
    pub fn new(context: &Rc<RefCell<Context>>) -> Result<Self, FennecError> {
        let context_borrowed = context.try_borrow()?;
        let functions = context_borrowed.functions();
        let surface_formats = unsafe {
            functions
                .instance_extensions()
                .surface()
                .get_physical_device_surface_formats(
                    *context_borrowed.physical_device(),
                    *context_borrowed.surface(),
                )
        }?;
        let format = surface_formats
            .iter()
            .find(|e| {
                e.format == PREFERRED_SURFACE_FORMAT && e.color_space == PREFERRED_COLOR_SPACE
            })
            .map(|e| Ok(e))
            .unwrap_or_else(|| {
                surface_formats
                    .iter()
                    .find(|e| e.format == PREFERRED_SURFACE_FORMAT)
                    .map(|e| Ok(e))
                    .unwrap_or_else(|| {
                        surface_formats.iter().nth(0).ok_or_else(|| {
                            FennecError::new(
                                "No surface formats available on this physical device... somehow?",
                            )
                        })
                    })
            })?;
        let surface_capabilities = unsafe {
            functions
                .instance_extensions()
                .surface()
                .get_physical_device_surface_capabilities(
                    *context_borrowed.physical_device(),
                    *context_borrowed.surface(),
                )?
        };
        let image_count =
            (surface_capabilities.max_image_count + surface_capabilities.min_image_count * 2) / 3;
        let resolution = match surface_capabilities.current_extent.width {
            std::u32::MAX => {
                let client_size = context_borrowed
                    .window()
                    .try_borrow()?
                    .client_size_pixels()?;
                vk::Extent2D {
                    width: client_size.0,
                    height: client_size.1,
                }
            }
            _ => surface_capabilities.current_extent,
        };
        let present_modes = unsafe {
            functions
                .instance_extensions()
                .surface()
                .get_physical_device_surface_present_modes(
                    *context_borrowed.physical_device(),
                    *context_borrowed.surface(),
                )?
        };
        let present_mode = present_modes
            .iter()
            .find(|e| **e == PREFERRED_PRESENT_MODE)
            .map(|e| Ok(e))
            .unwrap_or_else(|| {
                present_modes.iter().nth(0).ok_or_else(|| {
                    FennecError::new(
                        "No present modes available on this physical device... somehow?",
                    )
                })
            })?;
        let create_info = vk::SwapchainCreateInfoKHR::builder()
            .surface(*context_borrowed.surface())
            .min_image_count(image_count)
            .image_color_space(format.color_space)
            .image_format(format.format)
            .image_extent(resolution.clone())
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_DST)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(surface_capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(*present_mode)
            .clipped(true)
            .image_array_layers(1)
            .build();
        let swapchain = unsafe {
            functions
                .device_extensions()
                .swapchain()
                .create_swapchain(&create_info, None)
        }?;
        let images = unsafe {
            functions
                .device_extensions()
                .swapchain()
                .get_swapchain_images(swapchain)?
                .iter()
                .enumerate()
                .map(|(idx, image)| {
                    let mut wrapped = SwapchainImage::new(context, *image);
                    wrapped.set_name(&format!("Swapchain image {}", idx))?;
                    Ok(wrapped)
                })
                .handle_results()?
                .collect()
        };
        Ok(Self {
            swapchain: VKHandle::new(context, swapchain, false),
            swapchain_images: images,
        })
    }

    /// Get the swapchain images
    pub fn images(&self) -> &[SwapchainImage] {
        &self.swapchain_images
    }

    /// Acquire the next swapchain image to draw to
    pub fn acquire_next_image(
        &self,
        timeout_nanoseconds: Option<u64>,
        semaphore: Option<&Semaphore>,
        fence: Option<&Fence>,
    ) -> Result<u32, FennecError> {
        Ok(unsafe {
            self.context()
                .try_borrow()?
                .functions()
                .device_extensions()
                .swapchain()
                .acquire_next_image(
                    *self.handle().handle(),
                    timeout_nanoseconds.unwrap_or(std::u64::MAX),
                    semaphore
                        .map(|e| *e.handle().handle())
                        .unwrap_or(Default::default()),
                    fence
                        .map(|e| *e.handle().handle())
                        .unwrap_or(Default::default()),
                )
        }?
        .0)
    }

    /// Present one of the swapchain images
    pub fn present(
        &self,
        image_index: u32,
        queue: &Queue,
        semaphore: &Semaphore,
    ) -> Result<(), FennecError> {
        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(&[*semaphore.handle().handle()])
            .swapchains(&[*self.handle().handle()])
            .image_indices(&[image_index])
            .build();
        unsafe {
            self.context()
                .try_borrow()?
                .functions()
                .device_extensions()
                .swapchain()
                .queue_present(*queue.handle().handle(), &present_info)
        }?;
        Ok(())
    }
}

impl VKObject<vk::SwapchainKHR> for Swapchain {
    fn handle(&self) -> &VKHandle<vk::SwapchainKHR> {
        &self.swapchain
    }

    fn handle_mut(&mut self) -> &mut VKHandle<vk::SwapchainKHR> {
        &mut self.swapchain
    }

    fn object_type() -> vk::DebugReportObjectTypeEXT {
        vk::DebugReportObjectTypeEXT::SWAPCHAIN_KHR
    }
}

/// An image belonging to the swapchain
pub struct SwapchainImage {
    image: VKHandle<vk::Image>,
}

impl SwapchainImage {
    /// SwapchainImage factory method
    fn new(context: &Rc<RefCell<Context>>, swapchain_image: vk::Image) -> Self {
        Self {
            image: VKHandle::new(context, swapchain_image, true),
        }
    }
}

impl VKObject<vk::Image> for SwapchainImage {
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

impl Image for SwapchainImage {
    fn image_handle(&self) -> &VKHandle<vk::Image> {
        self.handle()
    }

    fn memory(&self) -> Option<&Memory> {
        None
    }
}
