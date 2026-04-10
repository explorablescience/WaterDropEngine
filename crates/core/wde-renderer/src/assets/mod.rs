//! Renderer assets module.
//!
//! This module contains:
//! - Definitions and loading logic for all assets used by the renderer, that is [`Mesh`](crate::assets::Mesh), [`Texture`](crate::assets::Texture), [`Buffer`](crate::assets::Buffer), [`Shader`](crate::assets::Shader) assets. Each asset type has its own loading logic and GPU preparation steps, defined in their respective submodules.
//! - The [`bindings`](crate::assets::bindings) submodule, which provides utilities for defining custom render resources and bindings.
//! - A set of default utility meshes, defined in the [`meshes`](crate::assets::meshes) submodule.
//!
//! # Asset loading
//! ## Default asset loading
//! Assets can be loaded with the default options using the `AssetServer` as usual:
//! ```
//! let texture_handle: Handle<Texture> = asset_server.load("res/my_texture.png");
//! ```
//!
//! ## Load with options
//! Some assets can be loaded with options, for example a texture can be loaded with specific sampler settings:
//! ```
//! let texture_handle: Handle<Texture> = assets_server.load_with_settings("res/my_texture.png",
//!     |settings: &mut TextureLoaderSettings| {
//!         settings.label = "my_texture".to_string();
//!         settings.format = TextureFormat::R8Unorm;
//!         settings.usages = TextureUsages::TEXTURE_BINDING;
//!     });
//! ```
//!
//! ## Custom asset creation
//! Assets can also be created from raw data, for example a texture can be created from raw pixel data:
//! ```
//! let texture_handle: Handle<Texture> = asset_server.add(Texture {
//!     label: "My Texture".to_string(),
//!     size: (256, 256),
//!     format: TextureFormat::Rgba8Unorm,
//!     usages: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
//!     data: vec![0u8; 256 * 256 * 4], // RGBA8 texture with all pixels set to transparent black
//!     ..Default::default()
//! });
//! ```
//!
//! # Render assets and render bindings
//! Assets defined in this module can be used as GPU resources in render bindings/pipelines by referencing them in the corresponding preparation systems.
//! The main types to be aware of are:
//! - [`RenderData`](crate::assets::bindings::RenderData): declares GPU resources (buffers/textures) and how they are recreated.
//! - [`RenderBinding`](crate::assets::bindings::RenderBinding): builds a bind group by referencing resources exposed by one or more [`RenderData`](crate::assets::bindings::RenderData) instances.
//!
//! ## RenderData
//! A [`RenderData`](crate::assets::bindings::RenderData) is used to allocate reusable GPU resources.
//! It is registered with [`RenderDataRegisterPlugin`](crate::assets::bindings::RenderDataRegisterPlugin), which:
//! - creates and prepares the corresponding [`GpuRenderData`](crate::assets::bindings::GpuRenderData),
//! - optionally recreates it when [`RenderData::recreate`](crate::assets::bindings::RenderData::recreate) returns `Some(true)`.
//!
//! Typical example (deferred textures):
//! ```
//! #[derive(Asset, Clone, TypePath, Default)]
//! pub struct DeferredTextures;
//! impl DeferredTextures {
//!     pub const ALBEDO_RESOLVED_IDX: u32 = 1;
//!     pub const DEMO_BUFFER: u32 = 2;
//! }
//!
//! impl RenderData for DeferredTextures {
//!     type Params = (SQuery<&'static Window>, SRes<Messages<SurfaceResized>>);
//!
//!     fn describe((window, _): &SystemParamItem<Self::Params>, builder: &mut RenderDataBuilder) {
//!         let size = {
//!             let window = window.single().unwrap();
//!             (window.resolution.physical_width(), window.resolution.physical_height())
//!         };
//!         let usages = TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING;
//!         builder
//!             .add_texture(Self::ALBEDO_RESOLVED_IDX,
//!                 Texture {
//!                     label: "pbr-albedo-resolved".to_string(),
//!                     size,
//!                     format: TextureFormat::Rgba8UnormSrgb,
//!                     usages,
//!                     ..Default::default()
//!                 },
//!             )
//!             .add_buffer(Self::DEMO_BUFFER,
//!                 Buffer {
//!                     label: "demo-buffer".to_string(),
//!                     size: 1024,
//!                     usage: BufferUsage::STORAGE | BufferUsage::COPY_DST,
//!                     content: None,
//!                 },
//!             );
//!     }
//!
//!     fn recreate((_, surface_resized): &SystemParamItem<Self::Params>) -> Option<bool> {
//!         Some(surface_resized.get_cursor().read(surface_resized).next().is_some())
//!     }
//! }
//!
//! pub struct DeferredTexturesPlugin;
//! impl Plugin for DeferredTexturesPlugin {
//!     fn build(&self, app: &mut App) {
//!         app.add_plugins(RenderDataRegisterPlugin::<DeferredTextures>::default());
//!     }
//! }
//! ```
//!
//! To later update the content of a render data buffer/texture, request [`ResRenderData`](crate::assets::bindings::ResRenderData) in a system and use the corresponding getters to access the GPU resource:
//! ```rust
//! fn update_buffer(mut deferred_textures: ResRenderData<DeferredTextures>, mut buffers: ResMut<GpuBuffers>) {
//!     let buffer = match buffer.iter().next() {
//!         Some((_, buffer)) => match buffer.get_buffer(DeferredTextures::DEMO_BUFFER) {
//!             Some(buffer) => buffer,
//!             None => return
//!         },
//!         _ => return
//!     };
//!
//!     // Update the buffer content with new data
//!     // (...)
//! }
//! ```
//!
//! ## RenderBinding
//! A [`RenderBinding`](crate::assets::bindings::RenderBinding) maps resources from [`GpuRenderData`](crate::assets::bindings::GpuRenderData) into a bind group layout + bind group pair ([`GpuRenderBinding`](crate::assets::bindings::GpuRenderBinding)).
//! It is registered with [`RenderBindingRegisterPlugin`](crate::assets::bindings::RenderBindingRegisterPlugin).
//!
//! Example: build a bind group from the resolved deferred textures.
//! ```
//!
//! #[derive(TypePath, Default, Clone, Asset)]
//! struct RenderBindingResolved;
//! impl RenderBinding for RenderBindingResolved {
//!     type Params = SRenderData<DeferredTextures>;
//!
//!     fn describe(
//!         deferred_textures: &SystemParamItem<Self::Params>,
//!         builder: &mut RenderBindingBuilder,
//!     ) {
//!         builder
//!             .add_texture_view(deferred_textures, DeferredTextures::ALBEDO_RESOLVED_IDX)
//!             .add_texture_sampler(deferred_textures, DeferredTextures::ALBEDO_RESOLVED_IDX);
//!     }
//!
//!     fn label(&self) -> &'static str { "deferred-textures-resolved-binding" }
//! }
//!
//! pub struct DeferredBindingPlugin;
//! impl Plugin for DeferredBindingPlugin {
//!     fn build(&self, app: &mut App) {
//!         app.add_plugins(RenderBindingRegisterPlugin::<RenderBindingResolved>::default());
//!     }
//! }
//! ```
//!
//! ## Referencing In Pipelines
//! To use a registered render binding in a pipeline preparation system, request [`SRenderBinding`](crate::assets::bindings::SRenderBinding) in the render asset params and use its `layout`/`bind_group`.
//! ```
//! type Params = (
//!     SRenderBinding<RenderBindingResolved>
//! );
//!
//! // In RenderPipelineDescriptor
//! bind_group_layouts: vec![
//!     // (...)
//!     render_binding.iter().next().map(|(_, d)| d.layout.clone()),
//! ],
//! ```
//!
//! In the sub-pass, bind groups must match that same ordering:
//! ```
//! // (...)
//! SubPassCommand::BindGroup(1, rbinding.iter().next().map(|(_, d)| d.bind_group.clone())),
//! ```
//!
//! ## Plugin Order
//! Register the underlying [`RenderData`](crate::assets::bindings::RenderData) plugin before the [`RenderBinding`](crate::assets::bindings::RenderBinding) plugin that depends on it.
//! This allows the binding preparation to retry until all dependent GPU resources are ready, and to react correctly when data are recreated.

mod asset;
mod bindings;
mod buffer;
mod mesh;
pub mod meshes;
mod shader;
mod texture;

use bevy::prelude::*;

pub use asset::*;
pub use bindings::*;
pub use buffer::*;
pub use mesh::*;
pub use meshes::*;
pub use shader::*;
pub use texture::*;

pub(crate) struct AssetsPlugin;
impl Plugin for AssetsPlugin {
    fn build(&self, app: &mut App) {
        // Setup the assets
        app.add_plugins(MaterialsPlugin)
            .init_asset_loader::<TextureLoader>()
            .init_asset::<Texture>()
            .init_asset_loader::<MeshLoader>()
            .init_asset::<Mesh>()
            .init_asset_loader::<ShaderLoader>()
            .init_asset::<Shader>()
            .init_asset::<Buffer>();

        // Add resource loaders to transfer the assets to the GPU
        app.add_plugins(RenderAssetsPlugin::<GpuMesh>::default())
            .add_plugins(RenderAssetsPlugin::<GpuTexture>::default())
            .add_plugins(RenderAssetsPlugin::<GpuBuffer>::default());

        // Register the components to the reflect system
        app.register_type::<Mesh3d>();
    }
}
