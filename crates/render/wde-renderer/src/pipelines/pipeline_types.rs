//! Descriptors for render and compute pipelines consumed by the pipeline manager.
//!
//! These are lightweight, CPU-side structs that mirror `wde-wgpu` pipeline
//! construction parameters, keeping the renderer crate decoupled from the exact
//! pipeline creation calls.
//!
//! ## Example: forward render pipeline descriptor
//! ```rust
//! use bevy::prelude::*;
//! use wde_renderer::pipelines::{RenderPipelineDescriptor, PushConstantDescriptor, RenderTopology, Face};
//! use wde_renderer::pipelines::{BindGroupLayout};
//! use wde_renderer::assets::Shader;
//!
//! fn build_descriptor(vert: Handle<Shader>, frag: Handle<Shader>, layouts: Vec<BindGroupLayout>) -> RenderPipelineDescriptor {
//!     RenderPipelineDescriptor {
//!         label: "forward-pipeline",
//!         vert: Some(vert),
//!         frag: Some(frag),
//!         render_targets: None, // default swapchain
//!         bind_group_layouts: layouts,
//!         push_constants: vec![PushConstantDescriptor { stages: ShaderStages::VERTEX, offset: 0, size: 16 }],
//!         topology: RenderTopology::TriangleList,
//!         cull_mode: Some(Face::Back),
//!         ..Default::default()
//!     }
//! }
//! ```

use bevy::{asset::Handle, ecs::prelude::*};
use wde_wgpu::{bind_group::BindGroupLayout, render_pipeline::{DepthStencilDescriptor, Face, ShaderStages, RenderTopology}, texture::TextureFormat};

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
    pub size: u32,
}

#[derive(Resource, Clone)]
/// Describes a render pipeline.
pub struct RenderPipelineDescriptor {
    /// Debug label forwarded to the GPU pipeline (default: "Render Pipeline").
    pub label: &'static str,
    /// Vertex shader handle (WGSL) to compile.
    pub vert: Option<Handle<Shader>>,
    /// Fragment shader handle (WGSL) to compile.
    pub frag: Option<Handle<Shader>>,
    /// Depth/stencil state for the pipeline.
    pub depth: DepthStencilDescriptor,
    /// Render targets; `None` renders to the swapchain surface by default.
    pub render_targets: Option<Vec<TextureFormat>>,
    /// Bind group layouts describing all resource bindings.
    pub bind_group_layouts: Vec<BindGroupLayout>,
    /// Push constant ranges exposed to shaders.
    pub push_constants: Vec<PushConstantDescriptor>,
    /// Primitive topology (default: TriangleList).
    pub topology: RenderTopology,
    /// Face culling mode (default: Back). `None` disables culling.
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
    /// Debug label forwarded to the GPU pipeline (default: "Compute Pipeline").
    pub label: &'static str,
    /// Compute shader handle (WGSL) to compile.
    pub comp: Option<Handle<Shader>>,
    /// Bind group layouts describing all resource bindings.
    pub bind_group_layouts: Vec<BindGroupLayout>,
    /// Push constant ranges exposed to the compute shader.
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
