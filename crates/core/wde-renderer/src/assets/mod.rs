//! Renderer assets module.
//!
//! This module contains:
//! - Definitions and loading logic for all assets used by the renderer, that is [`Mesh`](crate::assets::Mesh), [`Texture`](crate::assets::Texture), [`Buffer`](crate::assets::Buffer), [`Shader`](crate::assets::Shader) assets. Each asset type has its own loading logic and GPU preparation steps, defined in their respective submodules.
//! - The [`bindings`](crate::assets::bindings) submodule, which provides utilities for defining custom renderer bindings, such as custom materials. See the documentation of the [`bindings`](crate::assets::bindings) module for more details.
//! - A set of default utility meshes, defined in the [`meshes`](crate::assets::meshes) submodule.
//!
//! # Custom renderer bindings
//! For assets that require custom GPU bindings, such as custom materials [`Material`](crate::assets::bindings::Material), the [`bindings`](crate::assets::bindings) module provides utilities to define the asset and its corresponding GPU asset, as a [`RenderBinding`](crate::assets::bindings::RenderBinding) (or [`Material`](crate::assets::bindings::Material)) and a [`GpuRenderBinding`](crate::assets::bindings::GpuRenderBinding) (or [`GpuMaterial`](crate::assets::bindings::GpuMaterial)) respectively. See the documentation of the [`bindings`](crate::assets::bindings) module for more details.
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

mod asset;
pub mod bindings;
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
