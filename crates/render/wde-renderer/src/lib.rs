#![allow(clippy::just_underscores_and_digits)]
#![allow(clippy::type_complexity)]

pub mod prelude {
    pub use crate::RenderPlugin;
    pub use crate::assets::{LoadOp, StoreOp, Buffer, BufferBindingType, BufferUsage, GpuBuffer, RenderAssets, RenderAssetsPlugin, GpuMaterial, GpuTexture, PrepareAssetError, RenderAsset, MeshAsset, Mesh, GpuMesh, RenderPassBuilder, RenderPassColorAttachment, RenderPassDepth, CommandBuffer, ModelBoundingBox, TextureFormat, TextureUsages};
    pub use crate::core::{Extract, Render, RenderApp, RenderInstance, RenderSet, extract_macros::ExtractWorld, SwapchainFrame, window::SurfaceResized};
    pub use crate::components::TransformUniform;
    pub use crate::pipelines::{WgpuBindGroupLayout, BindGroup, BindGroupBuilder, BindGroupLayout, ShaderStages, CachedPipelineIndex, DepthStencilDescriptor, PipelineManager, RenderPipelineDescriptor, CachedPipelineStatus, Vertex, CompareFunction, Face, RenderTopology, BindGroupLayoutBuilder};
    pub use crate::assets::{Material, MaterialBuilder, MaterialsPluginRegister, Texture, TextureLoaderSettings};
    pub use crate::passes::{render_graph::{RenderGraph, RenderPass}, depth::{DepthTexture, DepthTextureLayout}};
    pub use crate::utils::Color;
}

pub mod assets;
pub mod pipelines;
pub mod components;
pub mod core;
pub mod passes;
pub mod utils;

use core::RenderCorePlugin;

use assets::AssetsPlugin;
use bevy::{app::{App, Plugin}, log::info};

pub struct RenderPlugin;
impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        // First, add the renderer plugin
        app.add_plugins(RenderCorePlugin);

        // Register the scene plugin
        app.add_plugins(AssetsPlugin);
    }

    fn finish(&self, _app: &mut App) {
        info!("Render plugin initialized.");
    }
}
