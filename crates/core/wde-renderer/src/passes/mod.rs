//! This module contains the render graph that describes the render logic of the engine.
//!
//! A frame is rendered by executing a render graph, which is made of a set of [`RenderPass`] that execute in a specific order. Each render pass describes a unique render stage (that is, the list of the attachments, load ops, etc.).
//! Each render pass is composed of muliple [`RenderSubPass`] that execute sequentially and describe the actual rendering logic (setting render or compute pipelines, bind groups, vertex buffers, draw calls, etc.).
//!
//! # Examples
//! ## Render pass
//! To define a render pass, you need to implement the [`RenderPass`] trait and its `describe` method, which returns a [`RenderPassDesc`] describing the pass attachments, load ops, etc. For example:
//! ```rust
//! pub struct DemoRenderPass;
//! impl RenderPass for DemoRenderPass {
//!     type Params = SBinding<DepthTexture>;
//!
//!     fn describe(depth_texture: &SystemParamItem<Self::Params>) -> RenderPassDesc {
//!         RenderPassDesc {
//!            attachments_depth: Some(RenderPassDescDepthAttachment {
//!                 texture: depth_texture.iter().next().and_then(|(_, t)| t.get_texture(0)),,
//!                 load: LoadOp::Clear(1.0),
//!                 ..default()
//!             }),
//!             ..default()
//!         }
//!     }
//!     fn id() -> RenderPassId { 100 }
//!     fn label() -> &'static str { "demo" }
//! }
//! ```
//! Here, we used the [`crate::prelude::SBinding`] system parameter to access the depth texture of a [crate::prelude::RenderBinding] resource. The `id` method returns a unique binding ID used to determine the execution order of the render passes (lower IDs execute first).
//! To register this render pass in the render graph , you can use the `add_pass` method of the [`RenderGraph`] resource in a plugin:
//! ```rust
//! app.get_sub_app_mut(RenderApp).unwrap().world_mut()
//!    .get_resource_mut::<RenderGraph>().unwrap()
//!    .add_pass::<DemoRenderPass>();
//! ```
//!
//! ## Render pipeline
//! Then, as each sub-pass requires a render pipeline, you need to define a render pipeline asset that implements the [`crate::prelude::RenderAsset`] trait, which describes how to prepare the pipeline from a source asset. For example:
//! ```rust
//! #[derive(TypePath, Default, Clone, Debug)]
//! pub struct DemoRenderPipeline(pub CachedPipelineIndex);
//! impl RenderAsset for DemoRenderPipeline {
//!     type SourceAsset = RenderPipelineAsset<DemoRenderPipeline>;
//!     type Params = (SRes<AssetServer>, SResMut<PipelineManager>);
//!
//!     fn prepare(
//!         _asset: Self::SourceAsset,
//!         (assets_server, pipeline_manager): &mut bevy::ecs::system::SystemParamItem<Self::Params>,
//!     ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
//!         Ok(DemoRenderPipeline(pipeline_manager.create_render_pipeline(RenderPipelineDescriptor {
//!             label: "demo-pipeline",
//!             vert: Some(assets_server.load(".../vert.wgsl")),
//!             frag: Some(assets_server.load(".../frag.wgsl")),
//!             bind_group_layouts: vec![DemoBindGroup::layout()],
//!             ..Default::default()
//!         })?))
//!     }
//! }
//! ```
//!
//! To register this render pipeline asset, you can use the `RenderPipelinePluginRegister` in a plugin
//! ```rust
//! app.add_plugins(RenderPipelinePluginRegister::<DemoRenderPipeline>::default());
//! ```
//! which will prepare the pipeline and make it available in the render world.
//!
//! ## Render sub-pass
//! Finally, you can define a render sub-pass that uses this pipeline and executes some draw calls. To do this, you need to implement the [`RenderSubPass`] trait and its `describe` method, which returns a [`RenderSubPassDesc`] describing the commands to execute in this sub-pass (see [`SubPassCommand`] for the list of available commands). For example:
//! ```rust
//! pub struct DemoSubRenderPass;
//! impl RenderSubPass for DemoSubRenderPass {
//!     type Params = (SRes<RenderAssets<DemoRenderPipeline>>, SRes<PostProcessingMesh>);
//!
//!     fn describe((pipeline, mesh): &SystemParamItem<Self::Params>) -> RenderSubPassDesc {
//!         RenderSubPassDesc(vec![
//!             SubPassCommand::Pipeline(Some(pipeline.iter().next().map(|(_, p)| p.0)).flatten()),
//!             SubPassCommand::Mesh(mesh.0.as_ref().map(|h| h.id())),
//!             SubPassCommand::DrawBatches(vec![DrawCommandsBatch {
//!                 index_range: 0..6,
//!                 ..Default::default()
//!             }])
//!         ])
//!     }
//!     fn label() -> &'static str { "demo" }
//! }
//! ```
//! Finally, you can register this sub-pass to the render pass using the `add_sub_pass` method of the [`RenderGraph`] resource:
//! ```rust
//! app.get_sub_app_mut(RenderApp).unwrap().world_mut()
//!   .get_resource_mut::<RenderGraph>().unwrap()
//!   .add_sub_pass::<DemoSubRenderPass, DemoRenderPass>();
//! ```
//!
//! # Custom rendering
//! ## Custom render commands
//! If the available commands in the `SubPassCommand` enum are not sufficient to describe your rendering logic, you can also use the `Custom` command, which takes a closure with access to the render world and the render pass encoder, allowing you to execute any custom rendering logic you want. For example:
//! ```rust
//! SubPassCommand::Custom(draw_custom)
//!```
//! where `draw_custom` is defined as:
//! ```rust
//! fn draw_custom<'pass>(world: &'pass World, render_pass: &mut RenderPassInstance<'pass>) {
//!    // Custom rendering logic here, with access to the render world and the render pass encoder
//! }
//! ```
//!
//! ## Custom render passes
//! If you want to execute a completely custom render pass, you can implement the [`RenderPass::custom_render`] method, which gives you access to the render world and a command buffer to execute commands directly without the restrictions of a render pass encoder. For example:
//! ```rust
//! pub struct CustomRenderPass;
//! impl RenderPass for CustomRenderPass {
//!    type Params = ();
//!
//!    fn describe(_params: &SystemParamItem<Self::Params>) -> RenderPassDesc { RenderPassDesc::default() }
//!
//!    fn custom_render(world: &mut World, command_buffer: &mut CommandBuffer) {
//!       // Custom render pass logic here, with access to the render world and a command buffer to execute commands directly without the restrictions of a render pass encoder
//!    }
//! }
//! ```

mod pipeline_manager;
mod pipeline_register_plugin;
mod pipeline_types;
mod render_graph;

// Reexport wgpu types
pub use wde_wgpu::pipelines::{
    BlendComponent, BlendFactor, BlendOperation, BlendState, ColorWrites, CompareFunction,
    DepthDescriptor, Face, RenderTopology, ShaderStages, StencilFaceState, StencilOperation,
    StencilState
};
pub use wde_wgpu::vertex::Vertex;

pub use pipeline_manager::*;
pub use pipeline_register_plugin::*;
pub use pipeline_types::*;
pub use render_graph::*;
