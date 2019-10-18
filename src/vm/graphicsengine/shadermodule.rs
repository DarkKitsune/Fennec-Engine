use super::vkobject::{VKHandle, VKObject};
use super::Context;
use crate::error::FennecError;
use ash::version::DeviceV1_0;
use ash::vk;
use spirv_reflect::ShaderModule as SPIRV;
use std::cell::RefCell;
use std::io::Read;
use std::rc::Rc;

/// Limit shaders to 100kb
pub const MAX_SHADER_SIZE: usize = 1024 * 100;

/// A SPIR-V shader module
pub struct ShaderModule {
    shader_module: VKHandle<vk::ShaderModule>,
    spirv: SPIRV,
}

impl ShaderModule {
    /// Factory method
    pub fn new(
        context: &Rc<RefCell<Context>>,
        source: &mut impl Read,
    ) -> Result<Self, FennecError> {
        // Read SPIR-V code
        let mut spv_code = Code {
            code_u8: [0u8; MAX_SHADER_SIZE],
        };
        let data_length = source.read(unsafe { &mut spv_code.code_u8 })?;
        if data_length % 4 != 0 {
            return Err(FennecError::new(
                "Shader source length is not a multiple of 4",
            ));
        }
        // Create reflection shader module
        let spirv =
            spirv_reflect::create_shader_module(unsafe { &spv_code.code_u8[0..data_length] })?;
        // Set create info
        let create_info = vk::ShaderModuleCreateInfo::builder()
            .code(unsafe { &spv_code.code_u32[0..data_length / 4] });
        // Create shader module
        let shader_module = unsafe {
            context
                .try_borrow()?
                .logical_device()
                .create_shader_module(&create_info, None)
        }?;
        // Return shader module
        Ok(Self {
            shader_module: VKHandle::new(context, shader_module, false),
            spirv,
        })
    }

    pub fn entry_point(&self) -> String {
        self.spirv.get_entry_point_name()
    }
}

impl VKObject<vk::ShaderModule> for ShaderModule {
    fn handle(&self) -> &VKHandle<vk::ShaderModule> {
        &self.shader_module
    }

    fn handle_mut(&mut self) -> &mut VKHandle<vk::ShaderModule> {
        &mut self.shader_module
    }

    fn object_type() -> vk::DebugReportObjectTypeEXT {
        vk::DebugReportObjectTypeEXT::SHADER_MODULE
    }

    fn set_children_names(&mut self) -> Result<(), FennecError> {
        Ok(())
    }
}

/// Represents SPIR-V shader code in binary form
union Code {
    code_u8: [u8; MAX_SHADER_SIZE],
    code_u32: [u32; MAX_SHADER_SIZE],
}

// TODO: Implement this, and make validating required before using
fn _validate_spirv(_spirv: &SPIRV) -> Result<(), FennecError> {
    Ok(())
}
