use super::vkobject::{VKHandle, VKObject};
use super::Context;
use crate::error::FennecError;
use ash::version::DeviceV1_0;
use ash::vk;
use std::cell::RefCell;
use std::rc::Rc;
//use std::mem::size_of;
use crate::iteratorext::IteratorResults;
use itertools::Itertools;

/// A graphics pipeline
pub struct GraphicsPipeline {
    pipeline: VKHandle<vk::Pipeline>,
}

impl GraphicsPipeline {
    /// GraphicsPipeline factory method
    pub fn new(
        context: &Rc<RefCell<Context>>,
        vertex_input_bindings: &[VertexInputBinding],
        topology: vk::PrimitiveTopology,
        stages: &[vk::PipelineShaderStageCreateInfo],
        viewports: &[Viewport],
        culling_mode: CullingState,
        depth_mode: &DepthState,
        blend_mode: &BlendState,
        advanced_settings: Option<AdvancedGraphicsPipelineSettings>,
    ) -> Result<Self, FennecError> {
        let advanced_settings = advanced_settings.unwrap_or_default();
        // Vertex input bindings
        let vertex_binding_descriptions = vertex_input_bindings
            .iter()
            .enumerate()
            .map(|(index, binding_info)| {
                vk::VertexInputBindingDescription::builder()
                    .binding(index as u32)
                    .stride(binding_info.stride)
                    .input_rate(binding_info.rate)
                    .build()
            })
            .collect::<Vec<vk::VertexInputBindingDescription>>();
        // Vertex input attributes
        let vertex_attribute_descriptions = vertex_input_bindings
            .iter()
            .enumerate()
            .map(|(binding_index, binding_info)| {
                binding_info
                    .attributes
                    .iter()
                    .map(|attribute| {
                        vk::VertexInputAttributeDescription::builder()
                            .binding(binding_index as u32)
                            .location(attribute.shader_binding_location)
                            .format(attribute.format.into())
                            .offset(attribute.offset)
                            .build()
                    })
                    .collect::<Vec<vk::VertexInputAttributeDescription>>()
            })
            .concat();
        // Vertex input state
        let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(&vertex_binding_descriptions)
            .vertex_attribute_descriptions(&vertex_attribute_descriptions)
            .build();
        // Input assembly state
        let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(topology)
            .primitive_restart_enable(false)
            .build();
        // Viewport state
        let vk_viewports = viewports
            .iter()
            .enumerate()
            .map(|(index, viewport)| {
                // Verify that the dimensions are greater than 0
                if viewport.width < 0.00001 {
                    return Err(FennecError::new(format!(
                        "Viewport {} has an invalid width; must be greater than 0",
                        index
                    )));
                }
                if viewport.height < 0.00001 {
                    return Err(FennecError::new(format!(
                        "Viewport {} has an invalid height; must be greater than 0",
                        index
                    )));
                }
                // Build viewport
                Ok(vk::Viewport::builder()
                    .x(viewport.x)
                    .y(viewport.y)
                    .width(viewport.width)
                    .height(viewport.height)
                    .min_depth(viewport.min_depth)
                    .max_depth(viewport.max_depth)
                    .build())
            })
            .handle_results()?
            .collect::<Vec<vk::Viewport>>();
        let scissors = viewports
            .iter()
            .enumerate()
            .map(|(index, viewport)| {
                // Verify that the scissor dimensions are greater than 0
                if viewport.scissor_extent.width < 1 {
                    return Err(FennecError::new(format!(
                        "Viewport {}'s scissor has an invalid width; must be greater than 0",
                        index
                    )));
                }
                if viewport.scissor_extent.height < 1 {
                    return Err(FennecError::new(format!(
                        "Viewport {}'s scissor has an invalid height; must be greater than 0",
                        index
                    )));
                }
                // Build viewport
                Ok(vk::Rect2D::builder()
                    .offset(viewport.scissor_offset)
                    .extent(viewport.scissor_extent)
                    .build())
            })
            .handle_results()?
            .collect::<Vec<vk::Rect2D>>();
        let viewport_state = vk::PipelineViewportStateCreateInfo::builder()
            .viewports(&vk_viewports)
            .scissors(&scissors)
            .build();
        // Rasterization state
        let rasterization_state = vk::PipelineRasterizationStateCreateInfo::builder()
            .depth_clamp_enable(advanced_settings.enable_depth_clamp.unwrap_or(false))
            .rasterizer_discard_enable(advanced_settings.disable_rasterization.unwrap_or(false))
            .polygon_mode(match topology {
                vk::PrimitiveTopology::LINE_LIST => vk::PolygonMode::LINE,
                vk::PrimitiveTopology::LINE_LIST_WITH_ADJACENCY => vk::PolygonMode::LINE,
                vk::PrimitiveTopology::LINE_STRIP => vk::PolygonMode::LINE,
                vk::PrimitiveTopology::LINE_STRIP_WITH_ADJACENCY => vk::PolygonMode::LINE,
                vk::PrimitiveTopology::POINT_LIST => vk::PolygonMode::POINT,
                _ => vk::PolygonMode::FILL,
            })
            .cull_mode(if culling_mode.enable {
                vk::CullModeFlags::BACK
            } else {
                vk::CullModeFlags::NONE
            })
            .front_face(culling_mode.front_face)
            .depth_bias_enable(advanced_settings.depth_bias.unwrap_or_default().enable)
            .depth_bias_constant_factor(
                advanced_settings
                    .depth_bias
                    .unwrap_or_default()
                    .constant_factor,
            )
            .depth_bias_clamp(advanced_settings.depth_bias.unwrap_or_default().clamp)
            .depth_bias_slope_factor(
                advanced_settings
                    .depth_bias
                    .unwrap_or_default()
                    .slope_factor,
            )
            .line_width(advanced_settings.line_width.unwrap_or(1.0))
            .build();
        // Depth/stencil state
        let depth_stencil_state = vk::PipelineDepthStencilStateCreateInfo::builder()
            .depth_test_enable(depth_mode.enable_test)
            .depth_write_enable(depth_mode.enable_write)
            .depth_compare_op(depth_mode.compare_op)
            .depth_bounds_test_enable(depth_mode.enable_bounds_test)
            .stencil_test_enable(depth_mode.enable_stencil_test)
            .front(depth_mode.stencil_front)
            .back(depth_mode.stencil_back)
            .min_depth_bounds(depth_mode.bounds_min)
            .max_depth_bounds(depth_mode.bounds_max)
            .build();
        // Color blend state
        let color_blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
            .logic_op_enable(blend_mode.enable_logic_op)
            .logic_op(blend_mode.logic_op)
            .attachments(&blend_mode.color_attachment_blend_functions)
            .blend_constants([
                blend_mode.blend_constant.0,
                blend_mode.blend_constant.1,
                blend_mode.blend_constant.2,
                blend_mode.blend_constant.3,
            ])
            .build();
        // Dynamic state
        let dynamic_state = vk::PipelineDynamicStateCreateInfo::builder()
            .dynamic_states(&advanced_settings.dynamic_states.unwrap_or_default())
            .build();
        // Set graphics pipeline create info
        let create_info = vk::GraphicsPipelineCreateInfo::builder()
            .flags(advanced_settings.flags.unwrap_or_default())
            .stages(stages)
            .vertex_input_state(&vertex_input_state)
            .input_assembly_state(&input_assembly_state)
            .viewport_state(&viewport_state)
            .rasterization_state(&rasterization_state)
            .depth_stencil_state(&depth_stencil_state)
            .color_blend_state(&color_blend_state)
            .dynamic_state(&dynamic_state)
            .build();
        // Create pipeline
        let possible_pipeline = unsafe {
            context
                .try_borrow()?
                .logical_device()
                .create_graphics_pipelines(Default::default(), &[create_info], None)
        };
        // Return pipeline
        match possible_pipeline {
            Ok(pipeline) => Ok(Self {
                pipeline: VKHandle::new(context, pipeline[0], false),
            }),
            Err((_pipeline, result)) => Err(FennecError::from(result)),
        }
    }
}

impl VKObject<vk::Pipeline> for GraphicsPipeline {
    fn handle(&self) -> &VKHandle<vk::Pipeline> {
        &self.pipeline
    }

    fn handle_mut(&mut self) -> &mut VKHandle<vk::Pipeline> {
        &mut self.pipeline
    }

    fn object_type() -> vk::DebugReportObjectTypeEXT {
        vk::DebugReportObjectTypeEXT::PIPELINE
    }
}

impl Pipeline for GraphicsPipeline {
    fn pipeline_handle(&self) -> &VKHandle<vk::Pipeline> {
        self.handle()
    }
}

/// Describes a vertex input binding and its attributes
pub struct VertexInputBinding {
    /// Stride of elements in input data
    pub stride: u32,
    /// Input rate for this binding
    pub rate: vk::VertexInputRate,
    /// Attributes in the binding
    pub attributes: Vec<VertexInputAttribute>,
}

/// Describes a vertex input attribute within a vertex input binding
pub struct VertexInputAttribute {
    /// Offset of the attribute in the input binding
    pub offset: u32,
    /// Which attribute binding location in the shader to bind to
    pub shader_binding_location: u32,
    /// Format of the attribute
    pub format: AttributeFormat,
}

/// Describes the format of an attribute
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum AttributeFormat {
    Float,
    Float2,
    Float3,
    Float4,
    Double,
    Double2,
    Double3,
    Double4,
    Int,
    Int2,
    Int3,
    Int4,
    Long,
    Long2,
    Long3,
    Long4,
}
/*
impl AttributeFormat {
    /// Gets the alignment of an attribute format using the std140 rules
    fn alignment_std140(self) -> usize {
        match self {
            AttributeFormat::Float => size_of::<f32>(),
            AttributeFormat::Float2 => size_of::<f32>() * 2,
            AttributeFormat::Float3 => AttributeFormat::Float4.alignment_std140(),
            AttributeFormat::Float4 => size_of::<f32>() * 4,
            AttributeFormat::Double => size_of::<f64>(),
            AttributeFormat::Double2 => size_of::<f64>() * 2,
            AttributeFormat::Double3 => AttributeFormat::Double4.alignment_std140(),
            AttributeFormat::Double4 => size_of::<f64>() * 4,
            AttributeFormat::Int => size_of::<i32>(),
            AttributeFormat::Int2 => size_of::<i32>() * 2,
            AttributeFormat::Int3 => AttributeFormat::Int4.alignment_std140(),
            AttributeFormat::Int4 => size_of::<i32>() * 4,
            AttributeFormat::Long => size_of::<i64>(),
            AttributeFormat::Long2 => size_of::<i64>() * 2,
            AttributeFormat::Long3 => AttributeFormat::Long4.alignment_std140(),
            AttributeFormat::Long4 => size_of::<i64>() * 4,
        }
    }
}*/

impl Into<vk::Format> for AttributeFormat {
    fn into(self) -> vk::Format {
        match self {
            AttributeFormat::Float => vk::Format::R32_SFLOAT,
            AttributeFormat::Float2 => vk::Format::R32G32_SFLOAT,
            AttributeFormat::Float3 => vk::Format::R32G32B32_SFLOAT,
            AttributeFormat::Float4 => vk::Format::R32G32B32A32_SFLOAT,
            AttributeFormat::Double => vk::Format::R64_SFLOAT,
            AttributeFormat::Double2 => vk::Format::R64G64_SFLOAT,
            AttributeFormat::Double3 => vk::Format::R64G64B64_SFLOAT,
            AttributeFormat::Double4 => vk::Format::R64G64B64A64_SFLOAT,
            AttributeFormat::Int => vk::Format::R32_SINT,
            AttributeFormat::Int2 => vk::Format::R32G32_SINT,
            AttributeFormat::Int3 => vk::Format::R32G32B32_SINT,
            AttributeFormat::Int4 => vk::Format::R32G32B32A32_SINT,
            AttributeFormat::Long => vk::Format::R64_SINT,
            AttributeFormat::Long2 => vk::Format::R64G64_SINT,
            AttributeFormat::Long3 => vk::Format::R64G64B64_SINT,
            AttributeFormat::Long4 => vk::Format::R64G64B64A64_SINT,
        }
    }
}

/// Describes a viewport and scissor
pub struct Viewport {
    /// Viewport x
    pub x: f32,
    /// Viewport y
    pub y: f32,
    /// Viewport width
    pub width: f32,
    /// Viewport height
    pub height: f32,
    /// Viewport depth minimum
    pub min_depth: f32,
    /// Viewport depth maximum
    pub max_depth: f32,
    /// Scissor offset
    pub scissor_offset: vk::Offset2D,
    /// Scissor extent
    pub scissor_extent: vk::Extent2D,
}

/// Describes a backface culling mode
#[derive(Default, Copy, Clone)]
pub struct CullingState {
    /// Whether backface culling is enabled
    pub enable: bool,
    /// What to consider a front face (back faces will be culled)
    pub front_face: vk::FrontFace,
}

/// Describes a depth test/write mode
#[derive(Default, Copy, Clone)]
pub struct DepthState {
    /// Whether depth testing is enabled
    pub enable_test: bool,
    /// Whether depth writing is enabled
    pub enable_write: bool,
    /// Compare incoming fragment depth with existing fragment depth using this operator\
    ///     Incoming = left operand, existing = right operand\
    ///     (ex: CompareOp::LESS means to use incoming fragment if ``incoming < existing``)
    pub compare_op: vk::CompareOp,
    pub enable_bounds_test: bool,
    pub bounds_min: f32,
    pub bounds_max: f32,
    pub enable_stencil_test: bool,
    pub stencil_front: vk::StencilOpState,
    pub stencil_back: vk::StencilOpState,
}

/// Describes a blend mode
#[derive(Default, Clone)]
pub struct BlendState {
    /// Enable use of the logic op
    pub enable_logic_op: bool,
    /// The logic op
    pub logic_op: vk::LogicOp,
    /// Blend function to use for each corresponding color attachment in a subpass
    pub color_attachment_blend_functions: Vec<vk::PipelineColorBlendAttachmentState>,
    /// Blend constant color
    // TODO: v Change this to use a Color struct when one exists v
    pub blend_constant: (f32, f32, f32, f32),
}

/// Advanced settings to be used in pipeline factory methods
#[derive(Default, Clone)]
pub struct AdvancedGraphicsPipelineSettings {
    /// Various flags for the pipeline
    pub flags: Option<vk::PipelineCreateFlags>,
    /// Enable depth clamping? *(default=false)*
    pub enable_depth_clamp: Option<bool>,
    /// Disable rasterization? (stages are still performed) *(default=false)*
    pub disable_rasterization: Option<bool>,
    /// Depth bias
    pub depth_bias: Option<DepthBias>,
    /// Line render width *(default=1.0)*
    pub line_width: Option<f32>,
    /// Rasterization sample count *(default=TYPE_1)*
    pub sample_count: Option<vk::SampleCountFlags>,
    /// Pipeline states (settings) that can be changed through commands
    pub dynamic_states: Option<Vec<vk::DynamicState>>,
}

/// Describes a set of depth bias settings
#[derive(Default, Copy, Clone)]
pub struct DepthBias {
    /// Whether depth bias is enabled
    pub enable: bool,
    /// The depth bias constant factor
    pub constant_factor: f32,
    /// The maximum/minimum depth bias
    pub clamp: f32,
    /// Scalar factor applied to a fragment’s slope in depth bias calculations
    pub slope_factor: f32,
}

/// Trait for Vulkan pipelines
pub trait Pipeline {
    /// Gets the handle of the wrapped Vulkan pipeline
    fn pipeline_handle(&self) -> &VKHandle<vk::Pipeline>;
}
