use bevy::{asset::Handle, ecs::prelude::*};
use wde_wgpu::{bind_group::BindGroupLayout, render_pipeline::{DepthStencilDescriptor, Face, ShaderStages, RenderTopology}, texture::TextureFormat};

use crate::assets::Shader;

/// Describes a push constant that will be available to a shader.
/// Note: the size of the push constant must be a multiple of 4 and must not exceed 128 bytes.
#[derive(Clone)]
pub struct PushConstantDescriptor {
    /// The shader stages that the push constant will be available to.
    pub stages: ShaderStages,
    /// The offset in bytes that the push constant will start at.
    pub offset: u32,
    /// The size in bytes of the push constant (note: this must be a multiple of 4 and must not exceed 128 bytes).
    pub size: u32,
}

#[derive(Resource, Clone)]
/// Describes a render pipeline.
pub struct RenderPipelineDescriptor {
    /// The label of the pipeline for debugging (default: "Render Pipeline").
    pub label: &'static str,
    /// The vertex shader of the pipeline (default: None).
    pub vert: Option<Handle<Shader>>,
    /// The fragment shader of the pipeline (default: None).
    pub frag: Option<Handle<Shader>>,
    /// Describes the depth and stencil state of the pipeline.
    pub depth: DepthStencilDescriptor,
    /// The render targets of the pipeline. By default, the pipeline will render to the swap chain.
    pub render_targets: Option<Vec<TextureFormat>>,
    /// The bind group layouts that the pipeline will use.
    pub bind_group_layouts: Vec<BindGroupLayout>,
    /// The push constants that the pipeline will use.
    pub push_constants: Vec<PushConstantDescriptor>,
    /// The primitive topology that the pipeline will use (default: TriangleList).
    pub topology: RenderTopology,
    /// The culling mode that the pipeline will use (default: Back). None will disable culling.
    pub cull_mode: Option<Face>,
}
impl Default for RenderPipelineDescriptor {
    fn default() -> Self {
        Self {
            label: "Render Pipeline",
            vert: None,
            frag: None,
            depth: DepthStencilDescriptor::default(),
            render_targets: None,
            bind_group_layouts: vec![],
            push_constants: vec![],
            topology: RenderTopology::TriangleList,
            cull_mode: Some(Face::Back),
        }
    }
}


#[derive(Resource, Clone)]
/// Describes a compute pipeline.
pub struct ComputePipelineDescriptor {
    /// The label of the pipeline for debugging (default: "Compute Pipeline").
    pub label: &'static str,
    /// The compute shader of the pipeline (default: None).
    pub comp: Option<Handle<Shader>>,
    /// The bind group layouts that the pipeline will use.
    pub bind_group_layouts: Vec<BindGroupLayout>,
    /// The push constants that the pipeline will use.
    pub push_constants: Vec<PushConstantDescriptor>,
}
impl Default for ComputePipelineDescriptor {
    fn default() -> Self {
        Self {
            label: "Compute Pipeline",
            comp: None,
            bind_group_layouts: vec![],
            push_constants: vec![]
        }
    }
}
