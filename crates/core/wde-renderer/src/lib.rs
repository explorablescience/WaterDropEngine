//! `wde-renderer` is the Bevy-based rendering layer of WaterDropEngine.
//!
//! It wraps `wde-wgpu` with asset loaders (mesh/texture/buffer/material), a pipelined render
//! sub-app, render graph for composable passes, cached pipelines, and shared utilities like
//! depth textures and color space helpers. The overall design prioritizes explicit control,
//! decoupling from `wgpu`, and easy extension via custom passes and materials.
//!
//! # Architecture Overview
//!
//! ## Render Pipeline
//!
//! ```text
//! Main App (Simulation)          Render Sub-App (GPU)
//! ├─ main world                  ├─ Extract Phase
//! │  (components,                │   ├─ Read main world (Queries)
//! │   resources)                 │   ├─ Copy to render world
//! │                              │   └─ (meshes, transforms, etc.)
//! │                              │
//! ├─ Extract Schedule ─────────────→ Render World
//! │  (async boundary)            │   (render resources)
//! │                              │
//! └─ Main Schedule              ├─ Render Phase
//!    (simulation,               │   ├─ Build bind groups
//!     physics, input)           │   ├─ Execute render passes
//!                               │   ├─ Issue draw commands
//!                               │   └─ Present frame
//!                               │
//!                               └─ Swapchain
//! ```
//!
//! The [`core::RenderApp`] is a Bevy sub-app running in parallel with the main app,
//! allowing the Nth frame's simulation and (N-1)th frame's rendering to overlap.
//!
//! ## Component Breakdown
//!
//! - **Core** ([`core`]): `RenderApp` lifecycle, extraction machinery, surface setup,
//!   window/swapchain syncing, and schedule definitions (`Extract`, `Render`, `RenderSet`).
//!
//! - **Assets** ([`assets`]): CPU/GPU asset pairs and preparation pipeline:
//!   - `Texture` → `GpuTexture` (image loading + GPU upload)
//!   - [`assets::MeshAsset`] → [`assets::GpuMesh`] (vertex/index buffers)
//!   - [`assets::Buffer`] → [`assets::GpuBuffer`] (uniforms, storage, indirect)
//!   - [`assets::Shader`] (WGSL source)
//!   - [`assets::Material`] → `GpuMaterial` (custom bind groups + layouts)
//!   - Prep pipeline handles dependency ordering (materials wait for textures/buffers).
//!
//! - **Pipelines** ([`pipelines`]): Descriptors and caching:
//!   - `RenderPipelineDescriptor`: layout, shaders, depth, topology, culling.
//!   - [`pipelines::ComputePipelineDescriptor`]: shader + bind groups for compute.
//!   - `PipelineManager`: queues, builds, and caches pipelines; watches shader changes
//!     for hot-reload.
//!   - Helpers: `BindGroupLayout`, `BindGroupBuilder`, `ShaderStages`, etc.
//!
//! - **Passes** ([`passes`]): Render graph and depth handling:
//!   - [`passes::render_graph::RenderPass`]: trait for extract + render phases.
//!   - [`passes::render_graph::RenderGraph`]: ordered execution of passes by ID.
//!   - [`passes::depth`]: auto-sized depth texture, lifecycle, bind groups.
//!
//! - **Components** ([`components`]): GPU-facing data like `TransformUniform`.
//!
//! - **Utilities** ([`utils`]): Color space helpers (linear/sRGB conversion).
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use bevy::prelude::*;
//! use wde_renderer::prelude::*;
//!
//! fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         .add_plugins(RenderPlugin)
//!         .add_systems(Startup, setup)
//!         .run();
//! }
//!
//! fn setup(mut commands: Commands, assets: Res<AssetServer>) {
//!     // Load a cube mesh
//!     let mesh = assets.add(CubeMesh::from("cube", 1.0));
//!     commands.spawn((Mesh(mesh), Transform::default()));
//! }
//! ```
//!
//! # Common Patterns
//!
//! ## 1. Adding a Custom Render Pass
//!
//! Implement [`passes::render_graph::RenderPass`] and register it:
//!
//! ```rust,no_run
//! use bevy::prelude::*;
//! use wde_renderer::prelude::*;
//!
//! #[derive(Default)]
//! struct MyPass;
//! impl RenderPass for MyPass {
//!     fn extract(&self, main_world: &mut World, render_world: &mut World) {
//!         // Copy data from main_world to render_world
//!     }
//!     fn render(&self, render_world: &mut World) {
//!         // Issue draw calls using cached pipelines/bind groups
//!     }
//! }
//!
//! pub struct MyPlugin;
//! impl Plugin for MyPlugin {
//!     fn build(&self, app: &mut App) {
//!         let render_app = app.sub_app_mut(RenderApp);
//!         render_app.add_systems(
//!             crate::core::Render,
//!             |mut graph: ResMut<RenderGraph>| {
//!                 graph.add_pass::<MyPass>(10); // runs at order 10
//!             }
//!         );
//!     }
//! }
//! ```
//!
//! See [`passes::render_graph`] docs for full example with extraction and draw calls.
//!
//! ## 2. Creating a Custom Material
//!
//! Implement [`assets::Material`], describe buffers/textures, then register:
//!
//! ```rust,no_run
//! use bevy::prelude::*;
//! use wde_renderer::prelude::*;
//!
//! #[derive(Asset, TypePath, Clone)]
//! struct UnlitColor { color: [f32; 4] }
//!
//! impl Material for UnlitColor {
//!     fn describe(&self, builder: &mut MaterialBuilder) {
//!         let bytes = bytemuck::cast_slice(&self.color).to_vec();
//!         builder.add_buffer(0, ShaderStages::FRAGMENT, BufferBindingType::Uniform,
//!                           bytes.len(), Some(bytes));
//!     }
//!     fn label(&self) -> String { "unlit-color".into() }
//! }
//!
//! // In App::build:
//! // app.add_plugins(MaterialsPluginRegister::<UnlitColor>::default());
//! ```
//!
//! See `assets::material` docs and the `wde-pbr` crate for complete examples.
//!
//! ## 3. Setting Up a Render Pipeline
//!
//! Create a descriptor, queue it through `PipelineManager`, then use the cached index:
//!
//! ```rust,no_run
//! use bevy::prelude::*;
//! use wde_renderer::prelude::*;
//!
//! fn queue_pipeline(
//!     mut pm: ResMut<PipelineManager>,
//!     assets: Res<AssetServer>,
//! ) {
//!     let vert = assets.load("shaders/my_vert.wgsl");
//!     let frag = assets.load("shaders/my_frag.wgsl");
//!     let desc = RenderPipelineDescriptor {
//!         label: "my-pipeline",
//!         vert: Some(vert),
//!         frag: Some(frag),
//!         bind_group_layouts: vec![], // add your layouts
//!         ..Default::default()
//!     };
//!     let _pipeline_id = pm.create_render_pipeline(desc);
//! }
//! ```
//!
//! See [`pipelines`] module docs for full PBR-like and compute examples.
//!
//! ## 4. Accessing GPU Resources in a Render Pass
//!
//! Use render-world resources like `RenderAssets<GpuMesh>`, `PipelineManager`, `DepthTextureLayout`:
//!
//! ```rust,no_run
//! fn my_render(render_world: &mut World) {
//!     let meshes = render_world.get_resource::<RenderAssets<GpuMesh>>()
//!         .expect("missing GPU meshes");
//!     let pm = render_world.get_resource::<PipelineManager>()
//!         .expect("missing pipeline manager");
//!     let depth = render_world.get_resource::<DepthTextureLayout>()
//!         .expect("missing depth");
//!
//!     // Check if a pipeline is ready
//!     match pm.get_pipeline(0) {
//!         CachedPipelineStatus::OkRender(pipeline) => {
//!             // Use pipeline for draw calls
//!         }
//!         CachedPipelineStatus::Loading => {
//!             // Pipeline still loading; skip this frame
//!         }
//!         CachedPipelineStatus::Error => {
//!             // Pipeline failed to compile
//!         }
//!         _ => {}
//!     }
//! }
//! ```
//!
//! # Render Scheduling
//!
//! The render sub-app uses these schedules/sets:
//!
//! - **[`core::Extract`]**: Runs in the main world; copies data to render world. Executes
//!   asynchronously *between* main app frames if pipelined rendering is enabled.
//! - **[`core::Render`]** schedule with [`core::RenderSet`]:
//!   - `PrepareAssets`: Upload CPU assets to GPU.
//!   - `BindGroups`: Build bind groups from resources.
//!   - `Render`: Issue render passes and draw calls.
//!   - `Submit`: Present the frame.
//!
//! Custom passes use [`passes::render_graph::RenderGraph::add_pass`] to hook into the extract/render flow,
//! running after all earlier passes.
//!
//! # Module Navigation
//!
//! - **Want to add a render pass?** → See [`passes::render_graph`] and [`passes`] module docs.
//! - **Want to define a custom material?** → See the [`assets`] module docs.
//! - **Want to set up a pipeline?** → See [`pipelines`] module docs.
//! - **Want to load assets?** → See [`assets`] module docs.
//! - **Want render app / extraction details?** → See [`core`] module docs.
//!
//! # Full Examples
//!
//! - **PBR rendering**: See the `wde-pbr` crate (gbuffer + deferred lighting).
//! - **Gizmo rendering**: See the `wde-gizmos` crate (wireframe, depth pass).
//! - **Marching cubes**: See examples in the main `src/` (compute pipeline + forward pass).
//!
//! # Pipelined Rendering
//!
//! For multi-threaded extraction to improve CPU/GPU parallelism, add:
//! ```rust,no_run
//! use wde_renderer::core::render_multithread::PipelinedRenderingPlugin;
//! // app.add_plugins(PipelinedRenderingPlugin);
//! ```
//! See [`core::render_multithread`] for details.
#![allow(clippy::just_underscores_and_digits)]
#![allow(clippy::type_complexity)]

pub mod prelude {
    pub use crate::RenderPlugin;
    pub use crate::assets::{*, meshes::*};
    pub use crate::core::{Extract, Render, RenderApp, RenderInstance, RenderSet, extract_macros::ExtractWorld, SwapchainFrame, window::SurfaceResized};
    pub use crate::components::TransformUniform;
    pub use crate::pipelines::*;
    pub use crate::assets::{Material, MaterialBuilder, MaterialsPluginRegister, Texture, TextureLoaderSettings, Shader};
    pub use crate::passes::{render_graph::{RenderGraph, RenderPass}, depth_msaa::{DepthTextureMSAA}, depth::{DepthTexture}};
    pub use crate::utils::Color;
}


pub mod assets;
pub mod pipelines;
pub mod components;
pub mod core;
pub mod ssbos;
pub mod passes;
pub mod utils;

use core::RenderCorePlugin;

use assets::AssetsPlugin;
use wde_logger::prelude::*;
use bevy::prelude::*;

/** Multisample anti-aliasing sample count used throughout the renderer. */
pub const MSAA_SAMPLE_COUNT: u32 = 4;

pub struct RenderPlugin;
impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        // First, add the renderer plugin
        app.add_plugins(RenderCorePlugin);

        // Add the ssbo mesh plugin
        app.add_plugins(ssbos::SsboPlugin);

        // Register the scene plugin
        app.add_plugins(AssetsPlugin);
    }

    fn finish(&self, _app: &mut App) {
        info!("Render plugin initialized.");
    }
}
