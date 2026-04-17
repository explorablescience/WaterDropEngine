//! GLTF Loader for WaterDropEngine
//!
//! This crate provides functionality to load and parse glTF files, converting them into assets that can be used within the WaterDropEngine. It supports loading meshes, materials, and textures defined in glTF files and integrates them with the engine's rendering system.
//!
//! # Example
//! To load a glTF model, you can use the [`GltfLoader`](crate::GltfLoader) in your `setup` system as follows:
//! ```rust
//! if let Ok(gltf_model) = GltfLoader::load("models/model.gltf", &asset_server) {
//!     commands.spawn((
//!         Transform::from_translation(Vec3::ZERO).with_scale(Vec3::splat(1.0)),
//!         PbrModel(gltf_model.models)
//!     ));
//! }
//! ```
//! The rendering of the loaded model is then managed by the [`wde_pbr`](wde_pbr) crate, which handles the materials and shaders for the meshes.
#![allow(clippy::too_many_arguments)]
use wde_logger::prelude::*;

use bevy::prelude::*;
use wde_pbr::prelude::*;
use wde_renderer::prelude::*;

#[doc(hidden)]
pub mod prelude {
    pub use crate::GltfAsset;
    pub use crate::GltfError;
    pub use crate::GltfLoader;
}

mod accessor;
mod error;
mod loader;
mod material;
mod model;
mod parser;

pub use error::GltfError;

/// Representation of a 3D GLTF model asset.
/// This will spawn the model and return the parent entity ID.
/// See the [crate] documentation for usage examples.
#[derive(Asset, TypePath, Clone)]
pub struct GltfAsset {
    pub path: String,
    /// The list of parsed glTF models.
    /// Each model is represented by a mesh and its associated material.
    pub models: Vec<(Handle<Mesh>, Handle<PbrMaterial>)>
}

/// Manager to load glTF models into the Bevy world.
/// See the [crate] documentation for usage examples.
pub struct GltfLoader;
impl GltfLoader {
    /// Load a glTF file and register its models and materials into the Bevy world.
    /// Returns a loaded `GltfAsset` which contains handles to the meshes and materials.
    pub fn load(path: &str, asset_server: &AssetServer) -> Result<GltfAsset, GltfError> {
        debug!("Loading glTF model {}.", path);

        // Parse the glTF file
        let model = parser::parse_gltf(path)?;

        // Form and load the model into the Bevy world
        let (materials, meshes, bounding_boxes) = loader::form_models(&model)?;
        let gltf_asset = loader::load_models(
            &model.path,
            &materials,
            &meshes,
            &bounding_boxes,
            asset_server
        );
        Ok(gltf_asset)
    }

    /// Spawn the loaded glTF model into the Bevy world, returning the parent entity ID.
    pub fn spawn(commands: &mut Commands, gltf_asset: &GltfAsset) -> Entity {
        let parent_entity = commands
            .spawn((
                Name::new(format!("GLTF Model {}", gltf_asset.path)),
                Transform::default()
            ))
            .id();
        for (i, (mesh_handle, material_handle)) in gltf_asset.models.iter().enumerate() {
            commands.spawn((
                Name::new(format!(
                    "Mesh Entity {} for GLTF Model {}",
                    i, gltf_asset.path
                )),
                Transform::default(),
                Mesh3d(mesh_handle.clone()),
                PbrMaterial3d(material_handle.clone()),
                ChildOf(parent_entity)
            ));
        }
        parent_entity
    }
}
