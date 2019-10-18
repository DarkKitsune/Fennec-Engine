use super::vkobject::{VKHandle, VKObject};
use super::Context;
use crate::error::FennecError;
use ash::version::DeviceV1_0;
use ash::vk;
use std::cell::RefCell;
use std::rc::Rc;

/// An image sampler
pub struct Sampler {
    sampler: VKHandle<vk::Sampler>,
}

impl Sampler {
    /// Factory method
    pub fn new(
        context: &Rc<RefCell<Context>>,
        filters: Filters,
        address_modes: AddressModes,
        anisotropy_settings: AnisotropySettings,
        advanced_settings: &AdvancedSamplerSettings,
    ) -> Result<Self, FennecError> {
        // Set create info
        // TODO: Figure out what compare_op, mip_lod_bias, min_lod, max_lod
        // TODO: and unnormalized_coordinates are and implement them ones somehow
        let create_info = vk::SamplerCreateInfo::builder()
            .min_filter(filters.min)
            .mag_filter(filters.mag)
            .address_mode_u(address_modes.u)
            .address_mode_v(address_modes.v)
            .border_color(address_modes.border_color)
            .anisotropy_enable(anisotropy_settings.enabled)
            .max_anisotropy(anisotropy_settings.max)
            .mipmap_mode(advanced_settings.mipmap_mode);
        // Create sampler
        let sampler = unsafe {
            context
                .try_borrow()?
                .logical_device()
                .create_sampler(&create_info, None)
        }?;
        // Return sampler
        Ok(Self {
            sampler: VKHandle::new(context, sampler, false),
        })
    }
}

impl VKObject<vk::Sampler> for Sampler {
    fn handle(&self) -> &VKHandle<vk::Sampler> {
        &self.sampler
    }

    fn handle_mut(&mut self) -> &mut VKHandle<vk::Sampler> {
        &mut self.sampler
    }

    fn object_type() -> vk::DebugReportObjectTypeEXT {
        vk::DebugReportObjectTypeEXT::SAMPLER
    }

    fn set_children_names(&mut self) -> Result<(), FennecError> {
        Ok(())
    }
}

/// Describes min and mag filter modes for a sampler
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Filters {
    pub min: vk::Filter,
    pub mag: vk::Filter,
}

impl Default for Filters {
    fn default() -> Self {
        Self {
            min: vk::Filter::LINEAR,
            mag: vk::Filter::LINEAR,
        }
    }
}

/// Describes U and V address modes for a sampler
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct AddressModes {
    pub u: vk::SamplerAddressMode,
    pub v: vk::SamplerAddressMode,
    pub border_color: vk::BorderColor,
}

impl Default for AddressModes {
    fn default() -> Self {
        Self {
            u: vk::SamplerAddressMode::REPEAT,
            v: vk::SamplerAddressMode::REPEAT,
            border_color: vk::BorderColor::FLOAT_OPAQUE_BLACK,
        }
    }
}

/// Describes the anisotropy settings for a sampler
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct AnisotropySettings {
    pub enabled: bool,
    pub max: f32,
}

impl Default for AnisotropySettings {
    fn default() -> Self {
        Self {
            enabled: false,
            max: 0.0,
        }
    }
}

/// Describes advanced settings for a sampler
#[derive(Clone, Debug, PartialEq)]
pub struct AdvancedSamplerSettings {
    pub mipmap_mode: vk::SamplerMipmapMode,
}

impl Default for AdvancedSamplerSettings {
    fn default() -> Self {
        Self {
            mipmap_mode: vk::SamplerMipmapMode::LINEAR,
        }
    }
}
