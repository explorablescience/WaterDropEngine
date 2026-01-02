//! `wde-renderer` is the Bevy-based rendering layer of WaterDropEngine. It wraps `wde-wgpu` with
//! asset loaders (mesh/texture/buffer/material), a pipelined render app, render graph, cached
//! pipelines, and utilities like depth textures and colors.
//!
//! # Architecture
//! - **Sub-app**: [`core::RenderApp`] runs a dedicated render schedule (`RenderSet`) in parallel
//!   with Bevy's main app, extracting state into a render world.
//! - **Core systems**: [`core::RenderCorePlugin`] spins up the wgpu instance/surface, syncs window
//!   size (`SurfaceResized`), manages swapchain frames, and wires extract/render schedules.
//! - **Assets**: [`assets`] provide CPU/GPU pairs for [`Buffer`]/[`GpuBuffer`], [`Texture`]/[`GpuTexture`],
//!   [`MeshAsset`]/[`GpuMesh`], [`Shader`], and [`Material`] via the [`RenderAsset`] pipeline.
//! - **Pipelines**: [`pipelines::PipelineManager`] caches render pipelines described by
//!   [`RenderPipelineDescriptor`] and exposes bind group helpers from `wde-wgpu`.
//! - **Render graph**: [`passes::render_graph::RenderGraph`] stores ordered render passes that
//!   implement [`passes::render_graph::RenderPass`] (gbuffer, lighting, gizmos, etc.).
//! - **Depth**: [`passes::depth::DepthTexture`] auto-allocates a depth target and rebuilds it on
//!   resize; [`passes::depth::DepthTextureLayout`] offers a bind group for sampling depth.
//! - **Utilities**: [`utils::Color`] provides linear/sRGB helpers consistent across crates.
//!
//! # Quickstart (hello renderer)
//! ```rust,no_run
//! use bevy::prelude::*;
//! use wde_renderer::prelude::*;
//!
//! fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         .add_plugins(RenderPlugin) // sets up RenderApp, assets, depth, pipelines
//!         .add_systems(Startup, spawn_mesh)
//!         .run();
//! }
//!
//! fn spawn_mesh(mut commands: Commands, assets: Res<AssetServer>) {
//!     // Upload a unit cube and a flat color material (example from pbr/gizmos add-ons)
//!     let mesh = assets.add(meshes::CubeMesh::from("cube", 1.0));
//!     commands.spawn((
//!         Mesh(mesh),
//!         Transform::from_xyz(0.0, 0.0, -3.0),
//!     ));
//! }
//! ```
//!
//! # Core usage patterns
//! - Add your own render passes by implementing [`passes::render_graph::RenderPass`] and
//!   registering them in [`RenderGraph::add_pass`] with an execution order index.
//! - Construct materials by implementing [`assets::Material`] and registering via
//!   [`assets::MaterialsPluginRegister`]; the GPU builder caches bind group layouts for reuse.
//! - Pipelines use cached indices; always guard draws on `CachedPipelineStatus::OkRender` before
//!   binding pipelines/buffers.
//! - Use `RenderSet::BindGroups` to build bind groups once per frame; `RenderSet::Render` for
//!   issuing draw/dispatch; `RenderSet::Submit` for presentation.
//!
//! # Modules
//! - [`core`]: render app lifecycle, extraction, window/surface handling, schedules.
//! - [`assets`]: loaders + GPU preparation for meshes, textures, buffers, shaders, materials.
//! - [`pipelines`]: pipeline descriptors, caching, bind group layout builders, vertex layout.
//! - [`passes`]: depth texture management and render graph orchestration.
//! - [`components`]: GPU-facing transforms (`TransformUniform`).
//! - [`utils`]: color helpers.
//!
//! # Examples and further reading
//! - Use `render_graph.add_pass::<YourPass>(order)` to chain custom passes (e.g. post-process).
//! - The `assets::material` module shows how to build fallback textures and staged buffers for
//!   materials; mirror that pattern for new material types.
//! - For multi-threaded extraction, see [`core::render_multithread::PipelinedRenderingPlugin`].
#![allow(clippy::just_underscores_and_digits)]
#![allow(clippy::type_complexity)]

pub mod prelude {
    pub use crate::RenderPlugin;
    pub use crate::assets::{LoadOp, StoreOp, Buffer, BufferBindingType, BufferUsage, GpuBuffer, RenderAssets, RenderAssetsPlugin, GpuMaterial, GpuTexture, PrepareAssetError, RenderAsset, MeshAsset, Mesh, GpuMesh, RenderPassBuilder, RenderPassColorAttachment, RenderPassDepth, CommandBuffer, ModelBoundingBox, TextureFormat, TextureUsages, meshes::{CubeMesh, PlaneMesh}};
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
