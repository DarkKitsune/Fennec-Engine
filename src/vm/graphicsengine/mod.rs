pub mod framebuffer;
pub mod image;
pub mod imageview;
pub mod memory;
pub mod pipeline;
pub mod queue;
pub mod renderpass;
pub mod swapchain;
pub mod sync;
pub mod vkobject;

use crate::error::FennecError;
use crate::fwindow::FWindow;
use crate::iteratorext::IteratorResults;
use ash::extensions::ext::{DebugMarker as DebugMarkerExt, DebugReport as DebugReportExt};
use ash::extensions::khr::{
    Surface as SurfaceExt, Swapchain as SwapchainExt, Win32Surface as Win32SurfaceExt,
};
use ash::version::{DeviceV1_0, EntryV1_0, InstanceV1_0};
use ash::vk;
use ash::{Device, Entry, Instance};
use colored::Colorize;
use framebuffer::Framebuffer;
use glutin::os::windows::WindowExt;
use image::Image;
use queue::QueueFamilyCollection;
use renderpass::{RenderPass, Subpass};
use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use std::rc::Rc;
use swapchain::Swapchain;
use sync::{Fence, Semaphore};
use vkobject::VKObject;
use winapi::um::libloaderapi::GetModuleHandleW;

/// Fennec graphics engine
pub struct GraphicsEngine {
    context: Rc<RefCell<Context>>,
    queue_family_collection: QueueFamilyCollection,
    swapchain: Swapchain,
    image_available_semaphore: Semaphore,
    render_test: RenderTest,
}

impl GraphicsEngine {
    /// GraphicsEngine factory method
    pub fn new(window: &Rc<RefCell<FWindow>>) -> Result<Self, FennecError> {
        // Set up Vulkan context
        let (context, mut queue_family_collection) = create_context(window)?;
        // Set up queue family collection
        queue_family_collection.setup(&context)?;
        // Create and name swapchain
        let mut swapchain = Swapchain::new(&context)?;
        swapchain.set_name("Display swapchain")?;
        // Create and name image_available_semaphore
        let mut image_available_semaphore = Semaphore::new(&context)?;
        image_available_semaphore.set_name("Image available semaphore")?;
        // Create render test stage
        let render_test = RenderTest::new(&context, &mut queue_family_collection, &swapchain)?;
        // Return the graphics engine
        Ok(Self {
            context,
            queue_family_collection,
            swapchain,
            image_available_semaphore,
            render_test,
        })
    }

    /// Executes the draw event
    pub fn draw(&mut self) -> Result<(), FennecError> {
        // Acquire next swapchain image to draw to
        let image_index =
            self.swapchain
                .acquire_next_image(None, Some(&self.image_available_semaphore), None)?;
        // Submit render test stage
        let render_test_finished = self.render_test.submit(
            (
                &self.image_available_semaphore,
                vk::PipelineStageFlags::TOP_OF_PIPE,
            ),
            &self.queue_family_collection,
            image_index,
            None,
        )?;
        // Present swapchain image
        let present_queue = self
            .queue_family_collection
            .present()
            .queue_of_priority(1.0)
            .ok_or_else(|| FennecError::new("No present queues exist"))?;
        self.swapchain
            .present(image_index, present_queue, render_test_finished)?;
        Ok(())
    }

    pub fn stop(&self) -> Result<(), FennecError> {
        unsafe {
            self.context
                .try_borrow()?
                .logical_device()
                .device_wait_idle()
        }?;
        Ok(())
    }
}

pub struct RenderTest {
    pub render_pass: RenderPass,
    pub framebuffers: Vec<Framebuffer>,
    pub finished_semaphore: Semaphore,
}

impl RenderTest {
    const COMMAND_BUFFERS_NAME: &'static str = "render_test";

    pub fn new(
        context: &Rc<RefCell<Context>>,
        queue_family_collection: &mut QueueFamilyCollection,
        swapchain: &Swapchain,
    ) -> Result<Self, FennecError> {
        // Create render finished semaphore
        let mut finished_semaphore = Semaphore::new(context)?;
        finished_semaphore.set_name("RenderTest finished semaphore")?;
        // Create render pass and framebuffers
        let attachments = [vk::AttachmentDescription::builder()
            .format(swapchain.format())
            .initial_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .final_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .samples(vk::SampleCountFlags::TYPE_1)
            .build()];
        let subpasses = [Subpass {
            input_attachments: vec![],
            color_attachments: vec![vk::AttachmentReference::builder()
                .attachment(0)
                .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .build()],
            depth_stencil_attachment: None,
            preserve_attachments: vec![],
            dependencies: vec![],
        }];
        let mut render_pass = RenderPass::new(context, &attachments, &subpasses)?;
        render_pass.set_name("RenderTest render pass")?;
        // TODO: remove unnecessary maps and unnecessary lambdas in maps from entire project
        let framebuffers = swapchain
            .images()
            .iter()
            .enumerate()
            .map(|(index, image)| {
                let mut view = image.view(&image.range_color_basic(), None)?;
                view.set_name(&format!("RenderTest framebuffer {} image view", index))?;
                let mut framebuffer = Framebuffer::new(context, &render_pass, vec![view])?;
                framebuffer.set_name(&format!("RenderTest framebuffer {}", index))?;
                Ok(framebuffer)
            })
            .handle_results()?
            .collect::<Vec<Framebuffer>>();
        // Create command buffers
        let graphics_long_term = queue_family_collection
            .graphics_mut()
            .command_pools_mut()
            .unwrap()
            .long_term_mut();
        let mut buffers = graphics_long_term
            .create_command_buffers(Self::COMMAND_BUFFERS_NAME, swapchain.images().len() as u32)?;
        for (i, buffer) in buffers.iter_mut().enumerate() {
            let image = &swapchain.images()[i];
            let writer = buffer.begin(false, true)?;
            writer.pipeline_barrier(
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                None,
                None,
                None,
                Some(&[vk::ImageMemoryBarrier::builder()
                    .image(*image.image_handle().handle())
                    .old_layout(vk::ImageLayout::UNDEFINED)
                    .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .src_access_mask(Default::default())
                    .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                    .subresource_range(image.range_color_basic())
                    .build()]),
            )?;
            let pass = writer.begin_render_pass(
                &render_pass,
                &framebuffers[i],
                vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: swapchain.extent(),
                },
                &[vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.5, 0.7, 0.9, 1.0],
                    },
                }],
            )?;
            pass.end();
            writer.pipeline_barrier(
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                None,
                None,
                None,
                Some(&[vk::ImageMemoryBarrier::builder()
                    .image(*image.image_handle().handle())
                    .old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                    .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                    .dst_access_mask(vk::AccessFlags::MEMORY_READ)
                    .subresource_range(image.range_color_basic())
                    .build()]),
            )?;
        }
        Ok(Self {
            finished_semaphore,
            render_pass,
            framebuffers,
        })
    }

    pub fn submit(
        &self,
        wait_for: (&Semaphore, vk::PipelineStageFlags),
        queue_family_collection: &QueueFamilyCollection,
        image_index: u32,
        signaled_fence: Option<&Fence>,
    ) -> Result<&Semaphore, FennecError> {
        let graphics_family = queue_family_collection.graphics();
        let graphics_long_term = graphics_family.command_pools().unwrap().long_term();
        graphics_family.queue_of_priority(1.0).unwrap().submit(
            Some(&[
                graphics_long_term.command_buffers(Self::COMMAND_BUFFERS_NAME)?
                    [image_index as usize],
            ]),
            Some(&[wait_for]),
            Some(&[&self.finished_semaphore]),
            signaled_fence,
        )?;
        Ok(&self.finished_semaphore)
    }
}

pub struct Context {
    window: Rc<RefCell<FWindow>>,
    functions: Functions,
    instance: Instance,
    debug_report_callback: vk::DebugReportCallbackEXT,
    surface: vk::SurfaceKHR,
    physical_device: vk::PhysicalDevice,
    logical_device: Device,
}

impl Context {
    fn new(
        window: &Rc<RefCell<FWindow>>,
        functions: Functions,
        instance: Instance,
        debug_report_callback: vk::DebugReportCallbackEXT,
        surface: vk::SurfaceKHR,
        physical_device: vk::PhysicalDevice,
        logical_device: Device,
    ) -> Result<Self, FennecError> {
        Ok(Self {
            window: window.clone(),
            functions,
            instance,
            debug_report_callback,
            surface,
            physical_device,
            logical_device,
        })
    }

    /// Gets the window
    pub fn window(&self) -> &Rc<RefCell<FWindow>> {
        &self.window
    }

    /// Gets the window
    pub fn window_mut(&mut self) -> &mut Rc<RefCell<FWindow>> {
        &mut self.window
    }

    /// Gets the Vulkan function loaders
    pub fn functions(&self) -> &Functions {
        &self.functions
    }

    /// Gets the Vulkan instance
    pub fn instance(&self) -> &Instance {
        &self.instance
    }

    /// Gets the debug report callback
    pub fn debug_report_callback(&self) -> &vk::DebugReportCallbackEXT {
        &self.debug_report_callback
    }

    /// Gets the window surface
    pub fn surface(&self) -> &vk::SurfaceKHR {
        &self.surface
    }

    /// Gets the physical device
    pub fn physical_device(&self) -> &vk::PhysicalDevice {
        &self.physical_device
    }

    /// Gets the logical device
    pub fn logical_device(&self) -> &Device {
        &self.logical_device
    }
}

pub struct Functions {
    entry: Entry,
    instance_extensions: InstanceExtensions,
    device_extensions: DeviceExtensions,
}

impl Functions {
    /// Functions factory method
    fn new(
        entry: Entry,
        instance_extensions: InstanceExtensions,
        device_extensions: DeviceExtensions,
    ) -> Self {
        Self {
            entry,
            instance_extensions,
            device_extensions,
        }
    }

    /// Gets the vulkan entry functions
    pub fn entry(&self) -> &Entry {
        &self.entry
    }

    /// Gets the loaded instance extensions
    pub fn instance_extensions(&self) -> &InstanceExtensions {
        &self.instance_extensions
    }

    /// Get the loaded device extensions
    pub fn device_extensions(&self) -> &DeviceExtensions {
        &self.device_extensions
    }
}

pub struct InstanceExtensions {
    debug_report: DebugReportExt,
    surface: SurfaceExt,
    os_surface: Win32SurfaceExt,
}

impl InstanceExtensions {
    /// InstanceExtensions factory method
    fn new(entry: &Entry, instance: &Instance) -> Self {
        Self {
            debug_report: DebugReportExt::new(entry, instance),
            surface: SurfaceExt::new(entry, instance),
            os_surface: Win32SurfaceExt::new(entry, instance),
        }
    }

    /// Gets the debug report extension
    pub fn debug_report(&self) -> &DebugReportExt {
        &self.debug_report
    }

    /// Gets the surface extension
    pub fn surface(&self) -> &SurfaceExt {
        &self.surface
    }

    /// Gets the os surface extension
    pub fn os_surface(&self) -> &Win32SurfaceExt {
        &self.os_surface
    }
}

/// Loaded device extensions
pub struct DeviceExtensions {
    swapchain: SwapchainExt,
    debug_marker: DebugMarkerExt,
}

impl DeviceExtensions {
    /// DeviceExtensions factory method
    fn new(instance: &Instance, device: &Device) -> Self {
        Self {
            swapchain: SwapchainExt::new(instance, device),
            debug_marker: DebugMarkerExt::new(instance, device),
        }
    }

    /// Gets the swapchain extension
    pub fn swapchain(&self) -> &SwapchainExt {
        &self.swapchain
    }

    /// Gets the debug marker extension
    pub fn debug_marker(&self) -> &DebugMarkerExt {
        &self.debug_marker
    }
}

/// The debug report callback function
unsafe extern "system" fn debug_report_callback_func(
    flags: vk::DebugReportFlagsEXT,
    object_type: vk::DebugReportObjectTypeEXT,
    object: u64,
    _location: usize,
    message_code: i32,
    p_layer_prefix: *const c_char,
    p_message: *const c_char,
    _p_user_data: *mut c_void,
) -> u32 {
    let prefix = CStr::from_ptr(p_layer_prefix as *mut c_char).to_string_lossy();
    let message = CStr::from_ptr(p_message as *mut c_char).to_string_lossy();
    println!(
        "{}",
        format!(
            "[{}] {:?} #{}:{} (Object={:?}:{})",
            prefix, flags, message_code, message, object_type, object
        )
        .color(if flags.contains(vk::DebugReportFlagsEXT::ERROR) {
            "red"
        } else if flags.contains(vk::DebugReportFlagsEXT::WARNING)
            || flags.contains(vk::DebugReportFlagsEXT::PERFORMANCE_WARNING)
        {
            "yellow"
        } else {
            "cyan"
        })
    );
    0
}

/// Create a Vulkan instance
fn create_instance(entry: &Entry) -> Result<Instance, FennecError> {
    let engine_name = CString::new(crate::manifest::ENGINE_NAME).map_err(|err| {
        FennecError::from_error(
            format!(
                "Could not convert engine name {:?} to CString",
                crate::manifest::ENGINE_NAME
            ),
            Box::new(err),
        )
    })?;
    let application_info = vk::ApplicationInfo::builder()
        .api_version(vk_make_version!(1, 0, 0))
        .engine_name(&engine_name)
        .engine_version(
            crate::manifest::ENGINE_VERSION.0 << 26
                | crate::manifest::ENGINE_VERSION.1 << 16
                | crate::manifest::ENGINE_VERSION.2,
        )
        .application_name(&engine_name)
        .application_version(0)
        .build();

    let extensions = validate_instance_extension_availability(
        entry,
        &[
            SurfaceExt::name(),
            Win32SurfaceExt::name(),
            DebugReportExt::name(),
        ],
    )?;
    let extensions_raw = extensions
        .iter()
        .map(|e| e.as_ptr())
        .collect::<Vec<*const c_char>>();
    let layers = [CString::new("VK_LAYER_LUNARG_standard_validation")?];
    //validate_layer_availability(&layers)?;
    let layers_raw = layers
        .iter()
        .map(|e| e.as_ptr())
        .collect::<Vec<*const c_char>>();
    let instance_create_info = vk::InstanceCreateInfo::builder()
        .application_info(&application_info)
        .enabled_extension_names(&extensions_raw)
        .enabled_layer_names(&layers_raw)
        .build();
    unsafe { Ok(entry.create_instance(&instance_create_info, None)?) }
}

/// Validate if required instance extensions are available
fn validate_instance_extension_availability(
    entry: &Entry,
    extensions: &[&'static CStr],
) -> Result<Vec<&'static CStr>, FennecError> {
    let available = entry.enumerate_instance_extension_properties()?;
    let mut ret = Vec::new();
    for extension in extensions.iter() {
        let name_string = (*extension)
            .to_str()
            .map_err(|err| {
                FennecError::from_error(
                    format!("Cannot convert {:?} to a string slice", extension),
                    Box::new(err),
                )
            })?
            .to_owned();
        let unavailable = available
            .iter()
            .map(|e| {
                let mut first_zero = true;
                let available_name_chars = e
                    .extension_name
                    .iter()
                    .take_while(|e| {
                        let chr = **e;
                        if chr == 0 {
                            if first_zero {
                                first_zero = false;
                                true
                            } else {
                                false
                            }
                        } else {
                            true
                        }
                    })
                    .map(|e| *e as u8)
                    .collect::<Vec<u8>>();
                let available_name_string = CStr::from_bytes_with_nul(&available_name_chars)
                    .map_err(|err| {
                        FennecError::from_error(
                            "Could not convert layer name to CString",
                            Box::new(err),
                        )
                    })?
                    .to_str()
                    .map_err(|err| {
                        FennecError::from_error(
                            "Could not convert layer name CString to string slice",
                            Box::new(err),
                        )
                    })?
                    .to_owned();
                Ok(name_string == available_name_string)
            })
            .handle_results()?
            .find(|e| *e)
            .is_none();
        if unavailable {
            return Err(FennecError::new(format!(
                "Instance extension {:?} is not available",
                *extension
            )));
        }
        ret.push(*extension);
    }
    Ok(ret)
}

/// Create a debug report callback
fn create_debug_report_callback(
    instance_extensions: &InstanceExtensions,
) -> Result<vk::DebugReportCallbackEXT, FennecError> {
    let debug_report_callback_create_info = vk::DebugReportCallbackCreateInfoEXT::builder()
        .pfn_callback(Some(debug_report_callback_func))
        .flags(
            vk::DebugReportFlagsEXT::DEBUG
                | vk::DebugReportFlagsEXT::ERROR
                | vk::DebugReportFlagsEXT::INFORMATION
                | vk::DebugReportFlagsEXT::PERFORMANCE_WARNING
                | vk::DebugReportFlagsEXT::WARNING,
        )
        .build();
    Ok(unsafe {
        instance_extensions
            .debug_report
            .create_debug_report_callback(&debug_report_callback_create_info, None)?
    })
}

// TODO: make work with other platforms instead of only Win32
/// Creates a window surface
fn create_surface(
    instance_extensions: &InstanceExtensions,
    window: &FWindow,
) -> Result<vk::SurfaceKHR, FennecError> {
    let hwnd = window.window().get_hwnd();
    let hinstance = unsafe { GetModuleHandleW(std::ptr::null()) };
    let win32_surface_create_info = vk::Win32SurfaceCreateInfoKHR::builder()
        .hwnd(hwnd)
        .hinstance(hinstance as *const c_void)
        .build();
    unsafe {
        Ok(instance_extensions
            .os_surface
            .create_win32_surface(&win32_surface_create_info, None)?)
    }
}

/// Chooses a physical device
fn choose_physical_device(
    entry: &Entry,
    instance: &Instance,
    surface: vk::SurfaceKHR,
) -> Result<(vk::PhysicalDevice, QueueFamilyCollection), FennecError> {
    Ok(unsafe { instance.enumerate_physical_devices()? }
        .iter()
        .filter_map(|device| unsafe {
            let families = instance.get_physical_device_queue_family_properties(*device);
            if let Ok(success) =
                QueueFamilyCollection::new(entry, instance, *device, surface, families)
                    .map(|collection| (*device, collection))
            {
                Some(success)
            } else {
                None
            }
        })
        .nth(0)
        .ok_or_else(|| {
            FennecError::new(
                "Could not find a physical device with a working graphics queue family",
            )
        })?)
}

/// Creates a logical device
fn create_logical_device(
    instance: &Instance,
    physical_device: vk::PhysicalDevice,
    queue_family_collection: &QueueFamilyCollection,
) -> Result<Device, FennecError> {
    let extensions = [
        SwapchainExt::name().as_ptr(),
        DebugMarkerExt::name().as_ptr(),
    ];
    let queue_priorities = queue_family_collection.queue_priorities();

    let queue_create_infos = queue_priorities
        .iter()
        .map(|e| {
            vk::DeviceQueueCreateInfo::builder()
                .queue_family_index(e.0)
                .queue_priorities(&e.1)
                .build()
        })
        .collect::<Vec<vk::DeviceQueueCreateInfo>>();

    let features = vk::PhysicalDeviceFeatures::builder().build();
    let device_create_info = vk::DeviceCreateInfo::builder()
        .queue_create_infos(&queue_create_infos)
        .enabled_extension_names(&extensions)
        .enabled_features(&features)
        .build();
    println!("{:?}", device_create_info);
    let device = unsafe { instance.create_device(physical_device, &device_create_info, None)? };
    Ok(device)
}

/// Creates a graphics context
fn create_context(
    window: &Rc<RefCell<FWindow>>,
) -> Result<(Rc<RefCell<Context>>, QueueFamilyCollection), FennecError> {
    // Load Vulkan entry functions
    let entry = Entry::new()?;
    // Create instance
    let instance = create_instance(&entry)?;
    // Load instance extensions
    let instance_extensions = InstanceExtensions::new(&entry, &instance);
    // Create debug report callback
    let debug_report_callback = create_debug_report_callback(&instance_extensions)?;
    // Create window surface
    let window_borrowed = window.try_borrow()?;
    let surface = create_surface(&instance_extensions, &window_borrowed)?;
    // Choose a physical device to use and create a queue family collection
    let (physical_device, queue_family_collection) =
        choose_physical_device(&entry, &instance, surface)?;
    // Create logical device
    let logical_device =
        create_logical_device(&instance, physical_device, &queue_family_collection)?;
    // Load device extensions
    let device_extensions = DeviceExtensions::new(&instance, &logical_device);
    // Create context wrapping all of this stuff
    let context = Rc::new(RefCell::new(Context::new(
        &window,
        Functions::new(entry, instance_extensions, device_extensions),
        instance,
        debug_report_callback,
        surface,
        physical_device,
        logical_device,
    )?));
    // Return context and queue family collection
    Ok((context, queue_family_collection))
}
