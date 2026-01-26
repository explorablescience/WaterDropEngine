//! Render pass orchestration: depth setup and user-defined passes via a simple graph.
//!
//! # Architecture
//!
//! - `depth`: handles creation, resize, and bind-group wiring for a shared depth texture.
//! - `render_graph`: minimal render graph trait (`RenderPass`) to plug custom passes.
//!
//! A `RenderPass` is a pair of `extract` and `render` functions that together define
//! a rendering stage. The `extract` phase (running in the main world) copies data from
//! the main app into the render world (cameras, meshes, pipeline indices, etc.). The
//! `render` phase (running in the render world) issues GPU commands using that extracted data.
//!
//! Passes are executed in order by their numeric ID (lower first), so dependencies can be
//! expressed through ID ordering (e.g., gbuffer pass at ID 0, lighting at ID 10).
//!
//! # Depth Texture
//!
//! Depth resources are initialized automatically by `RendererPlugin`:
//! - `DepthTexture::create_texture` creates the surface-sized depth texture in Startup.
//! - `DepthTexture::resize_texture` recreates it when the window resizes.
//! - `DepthTexture::extract_texture` moves the handle to the render world.
//! - `DepthTextureLayout::build_bind_group` wires the bind group in the Render phase.
//!
//! Most passes can sample the depth bind group from the render world resource.
//!
//! # Simple Example: Gizmo Pass
//!
//! ```rust
//! use bevy::prelude::*;
//! use wde_renderer::passes::render_graph::{RenderGraph, RenderPass};
//!
//! #[derive(Default)]
//! struct GizmoPass;
//! impl RenderPass for GizmoPass {
//!     fn render(&self, _render_world: &mut World) {
//!         // Issue gizmo draw calls using cached pipelines/bind groups
//!     }
//! }
//!
//! fn add_pass(mut graph: ResMut<RenderGraph>) {
//!     graph.add_pass::<GizmoPass>(10); // runs after earlier passes
//! }
//! ```
//!
//! # Full Example: Custom Forward Pass with Depth
//!
//! A realistic pass extracts scene data, manages a bind group layout, and issues draw calls:
//!
//! ```rust
//! use bevy::prelude::*;
//! use wde_renderer::passes::render_graph::{RenderGraph, RenderPass};
//! use wde_renderer::pipelines::PipelineManager;
//! use wde_renderer::core::{Extract, RenderSet};
//! use wde_renderer::assets::GpuMesh;
//!
//! // 1. Define extracted data (moved from main world to render world)
//! #[derive(Resource, Default)]
//! struct ExtractedRenderables {
//!     meshes: Vec<(Handle<GpuMesh>, Transform)>,
//! }
//!
//! // 2. Extract from main world
//! fn extract_renderables(
//!     mut commands: Commands,
//!     query: Query<(&Handle<GpuMesh>, &Transform)>
//! ) {
//!     let mut extracted = ExtractedRenderables::default();
//!     for (mesh, transform) in &query {
//!         extracted.meshes.push((mesh.clone(), *transform));
//!     }
//!     commands.insert_resource(extracted);
//! }
//!
//! // 3. Define the render pass
//! #[derive(Default)]
//! struct ForwardPass;
//! impl RenderPass for ForwardPass {
//!     fn extract(&self, main_world: &mut World, render_world: &mut World) {
//!         // Use extract_renderables system
//!         let mut state = SystemState::<Query<(&Handle<GpuMesh>, &Transform)>>::new(main_world);
//!         let query = state.get(main_world);
//!         let mut extracted = ExtractedRenderables::default();
//!         for (mesh, transform) in &query {
//!             extracted.meshes.push((mesh.clone(), *transform));
//!         }
//!         render_world.insert_resource(extracted);
//!     }
//!
//!     fn render(&self, render_world: &mut World) {
//!         // 1. Get extracted data
//!         let extracted = render_world.get_resource::<ExtractedRenderables>()
//!             .expect("missing extracted renderables");
//!
//!         // 2. Get render instance, pipeline, and bind groups
//!         let render_instance = render_world.get_resource::<RenderInstance>()
//!             .expect("missing render instance");
//!         let pm = render_world.get_resource::<PipelineManager>()
//!             .expect("missing pipeline manager");
//!         let depth_layout = render_world.get_resource::<DepthTextureLayout>()
//!             .expect("missing depth layout");
//!
//!         // 3. Check pipeline is ready
//!         let pipeline_id = 0; // e.g., stored on a component
//!         if let CachedPipelineStatus::OkRender(pipeline) = pm.get_pipeline(pipeline_id) {
//!             // 4. For each renderable, issue draw call
//!             for (mesh_handle, transform) in &extracted.meshes {
//!                 // Get GPU mesh
//!                 let meshes = render_world.get_resource::<RenderAssets<GpuMesh>>()
//!                     .and_then(|m| m.get(mesh_handle));
//!
//!                 if let Some(gpu_mesh) = meshes {
//!                     // Issue draw: bind pipeline, bind groups, bind buffers, draw indexed
//!                     // (actual wgpu command buffer recording omitted for brevity)
//!                 }
//!             }
//!         }
//!     }
//! }
//!
//! // 4. Register the pass
//! pub struct CustomRenderPlugin;
//! impl Plugin for CustomRenderPlugin {
//!     fn build(&self, app: &mut App) {
//!         let render_app = app.sub_app_mut(wde_renderer::core::RenderApp);
//!         render_app.add_systems(wde_renderer::core::Extract, extract_renderables);
//!         render_app.add_systems(wde_renderer::core::Render, |mut graph: ResMut<RenderGraph>| {
//!             graph.add_pass::<ForwardPass>(5);
//!         });
//!     }
//! }
//! ```
//!
//! For complete examples, see the `wde-pbr` and `wde-gizmos` crates.

use bevy::prelude::*;
use depth_msaa::{DepthTextureMSAA};
use depth::{DepthTexture};

use crate::{assets::RenderAssetsPlugin, core::{Extract, Render, RenderApp, RenderSet}, passes::{depth_blit_pipeline::{DepthBlitRenderPipeline, DepthBlitRenderPipelineAsset, GpuDepthBlitRenderPipeline}, depth_blit_renderpass::DepthBlitRenderPass, depth_msaa::DepthMSAATextureLayout}, prelude::RenderGraph};

pub mod depth;
pub mod depth_msaa;
pub mod depth_blit_pipeline;
pub mod depth_blit_renderpass;
pub mod render_graph;

pub(crate) struct RendererPlugin;
impl Plugin for RendererPlugin {
    fn build(&self, app: &mut App) {
        // Add the depth MSAA texture to the app
        app
            .add_systems(Startup, DepthTextureMSAA::init)
            .add_systems(Update, DepthTextureMSAA::resize);
        app.get_sub_app_mut(RenderApp).unwrap()
            .add_systems(Extract, DepthTextureMSAA::extract)
            .init_resource::<DepthMSAATextureLayout>()
            .add_systems(Render, DepthMSAATextureLayout::build_bind_group.in_set(RenderSet::BindGroups));

        // Add the depth texture to the app
        app
            .add_systems(Startup, DepthTexture::create_texture)
            .add_systems(Update, DepthTexture::resize_texture);
        app.get_sub_app_mut(RenderApp).unwrap()
            .add_systems(Extract, DepthTexture::extract_texture);

        // Add the depth blit pipeline
        app
            .init_asset::<DepthBlitRenderPipelineAsset>()
            .add_plugins(RenderAssetsPlugin::<GpuDepthBlitRenderPipeline>::default());

        // Add the creation of the mesh for the depth blit pass
        app
            .init_resource::<DepthBlitRenderPass>()
            .add_systems(Startup, DepthBlitRenderPass::init);
        app.get_sub_app_mut(RenderApp).unwrap()
            .init_resource::<DepthBlitRenderPass>();

        // Add the depth blit render pass (always at binding 100)
        let mut render_graph = app.get_sub_app_mut(RenderApp).unwrap()
            .world_mut().get_resource_mut::<RenderGraph>().unwrap();
        render_graph.add_pass::<DepthBlitRenderPass>(100);
    }

    fn finish(&self, app: &mut App) {
        // Create the pipeline
        let pipeline = app.world_mut()
            .get_resource::<AssetServer>().unwrap().add(DepthBlitRenderPipelineAsset);
        app.get_sub_app_mut(RenderApp).unwrap().world_mut().spawn(DepthBlitRenderPipeline(pipeline));
    }
}
