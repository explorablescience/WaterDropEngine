//! Pipeline descriptors and cache utilities bridging `wde-renderer` to `wde-wgpu`.
//!
//! The `pipeline_types` module describes CPU-side pipeline descriptors; the
//! `pipeline_manager` queues and builds them into GPU pipelines. See below for
//! quick-start snippets, and refer to `wde-pbr` for fuller examples of forward
//! and deferred setups.
//!
//! ## Example: render pipeline (PBR-like layout)
//! ```rust
//! use bevy::prelude::*;
//! use wde_renderer::pipelines::{RenderPipelineDescriptor, PipelineManager, ShaderStages, RenderTopology, Face};
//! use wde_renderer::pipelines::BindGroupLayout;
//! use wde_renderer::assets::Shader;
//!
//! fn queue_pbr_like_pipeline(
//!     mut pm: ResMut<PipelineManager>,
//!     layouts: Res<MyLayouts>,
//!     assets: Res<AssetServer>
//! ) {
//!     let vert: Handle<Shader> = assets.load("pbr/gbuffer_vert.wgsl");
//!     let frag: Handle<Shader> = assets.load("pbr/gbuffer_frag.wgsl");
//!     let desc = RenderPipelineDescriptor {
//!         label: "pbr-gbuffer",
//!         vert: Some(vert),
//!         frag: Some(frag),
//!         bind_group_layouts: vec![
//!             layouts.globals.clone(), // camera, lighting buffers
//!             layouts.materials.clone(),
//!             layouts.textures.clone(),
//!         ],
//!         push_constants: vec![],
//!         topology: RenderTopology::TriangleList,
//!         cull_mode: Some(Face::Back),
//!         ..Default::default()
//!     };
//!     let _id = pm.create_render_pipeline(desc);
//! }
//! ```
//!
//! ## Example: compute pipeline (terrain/marching-cubes style)
//! ```rust
//! use bevy::prelude::*;
//! use wde_renderer::pipelines::{ComputePipelineDescriptor, PipelineManager};
//! use wde_renderer::pipelines::BindGroupLayout;
//! use wde_renderer::assets::Shader;
//!
//! fn queue_compute_pipeline(
//!     mut pm: ResMut<PipelineManager>,
//!     layouts: Res<MyComputeLayouts>,
//!     assets: Res<AssetServer>
//! ) {
//!     let comp: Handle<Shader> = assets.load("marching-cubes/spawn_terrain.comp.wgsl");
//!     let desc = ComputePipelineDescriptor {
//!         label: "terrain-spawn",
//!         comp: Some(comp),
//!         bind_group_layouts: vec![layouts.storage.clone(), layouts.uniforms.clone()],
//!         push_constants: vec![],
//!     };
//!     let _id = pm.create_compute_pipeline(desc);
//! }
//! ```
//!
//! For full layouts, bind groups, and shader contracts, see the pbr crate and
//! the per-file docs in `pipelines/pipeline_types.rs`.

mod pipeline_types;
mod pipeline_manager;

// Reexport types
pub use wde_wgpu::bind_group::{WgpuBindGroupLayout, BindGroupBuilder, BindGroupLayout, WgpuBindGroup as BindGroup, BindGroupLayoutBuilder};
pub use wde_wgpu::render_pipeline::{CompareFunction, Face, ShaderStages, RenderTopology, DepthDescriptor};
pub use wde_wgpu::vertex::Vertex;

pub use pipeline_types::*;
pub use pipeline_manager::*;
