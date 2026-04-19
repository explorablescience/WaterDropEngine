use bevy::{asset::Handle, ecs::prelude::*};
use wde_wgpu::{
    bind_group::BindGroupLayout,
    pipelines::{BlendState, ColorWrites},
    render_pipeline::{DepthDescriptor, Face, RenderTopology, ShaderStages},
    texture::TextureFormat
};

use crate::assets::Shader;

/// Describes a push constant that will be available to a shader.
/// Note: the size of the push constant must be a multiple of 4 and must not exceed 128 bytes.
#[derive(Clone)]
pub struct PushConstantDescriptor {
    /// Shader stages that can read the push constant.
    pub stages: ShaderStages,
    /// Byte offset from the start of the push constant buffer.
    pub offset: u32,
    /// Size in bytes (multiple of 4, up to 128).
    pub size: u32
}

/// Describes a render pipeline, including its shaders, resources and states.
#[derive(Resource, Clone)]
pub struct RenderPipelineDescriptor {
    pub label: &'static str,
    pub vert: Option<Handle<Shader>>,
    pub frag: Option<Handle<Shader>>,

    /// Depth/stencil state for the pipeline.
    pub depth: DepthDescriptor,
    /// Formats of the render targets this pipeline will render to. If `None`, the pipeline will render to the swapchain format.
    pub render_targets: Option<Vec<TextureFormat>>,

    /// Bind group layouts describing all resource bindings.
    pub bind_group_layouts: Vec<Option<BindGroupLayout>>,
    /// Push constant ranges exposed to shaders.
    pub push_constants: Vec<PushConstantDescriptor>,

    /// Primitive topology (default: TriangleList).
    pub topology: RenderTopology,
    /// Face culling mode (default: Back). `None` disables culling.
    pub cull_mode: Option<Face>,
    /// Blend state for the fragment shader (default: None, i.e. no blending).
    pub fragment_blend: Option<BlendState>,
    /// Color write mask for fragment outputs (default: all channels). This enable you to disable writing to certain color channels.
    pub color_write: ColorWrites,
    /// The sample count for multisampling (default: 1).
    pub sample_count: u32,
    /// Whether the pipeline should expect a vertex buffer. Useful for pipelines that render fullscreen quads without vertex buffers (default: true).
    pub vertex_buffer: bool
}
impl Default for RenderPipelineDescriptor {
    fn default() -> Self {
        Self {
            label: "Unknown Render Pipeline",
            vert: None,
            frag: None,
            depth: DepthDescriptor::default(),
            render_targets: None,
            bind_group_layouts: vec![],
            push_constants: vec![],
            topology: RenderTopology::TriangleList,
            cull_mode: Some(Face::Back),
            fragment_blend: Some(BlendState::REPLACE),
            color_write: ColorWrites::ALL,
            sample_count: 1,
            vertex_buffer: true
        }
    }
}

/// Describes a compute pipeline, including its shader and resources.
#[derive(Resource, Clone)]
pub struct ComputePipelineDescriptor {
    pub label: &'static str,
    pub comp: Option<Handle<Shader>>,

    /// Bind group layouts describing all resource bindings.
    pub bind_group_layouts: Vec<Option<BindGroupLayout>>,
    /// Push constant ranges exposed to the compute shader.
    pub push_constants: Vec<PushConstantDescriptor>
}
impl Default for ComputePipelineDescriptor {
    fn default() -> Self {
        Self {
            label: "Unknown Compute Pipeline",
            comp: None,
            bind_group_layouts: vec![],
            push_constants: vec![]
        }
    }
}
