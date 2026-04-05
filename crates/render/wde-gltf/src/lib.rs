//! GLTF (GL Transmission Format) model loader and parser for WaterDropEngine.
//! This crate provides functionality to load and parse glTF 2.0 files,
//! converting them into Bevy-compatible mesh and material assets for rendering.
//! 
//! # Features
//! - Load glTF files from disk.
//! - Parse glTF structures including buffers, meshes, accessors, and materials.
//! - Convert glTF materials to PBR materials compatible with WaterDropEngine's PBR system.
//! - Create mesh assets from glTF mesh data.
//! - Spawn loaded models into the Bevy world with appropriate transforms.
//!
//! # Example
//! The following example demonstrates how to load a glTF model and spawn it into the world.
//! ```rust,no_run
//! fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
//!     if let Ok(gltf_model) = GltfLoader::load("models/model.gltf", &asset_server) {
//!         GltfLoader::spawn(Transform::from_scale(Vec3::ONE * 10.0), commands, &gltf_model);
//!     }
//! }
//! ``` 
#![allow(clippy::too_many_arguments)]

use wde_logger::prelude::*;
use wde_renderer::prelude::*;
use wde_pbr::prelude::*;
use bevy::prelude::*;

pub mod prelude {
    pub use crate::GltfAsset;
    pub use crate::GltfLoader;
    pub use crate::GltfError;
}

mod model;
mod material;
mod parser;
mod loader;
mod accessor;
mod error;

pub use error::GltfError;

/// Representation of a 3D GLTF model asset.
/// 
/// # Fields
/// - `path`: The path to the glTF file.
/// - `models`: The list of parsed glTF models. Each model is represented by a mesh and its associated material.
///   Node transforms are baked directly into the mesh vertices.
/// 
/// # Example
/// ```rust,no_run
/// let gltf_asset = GltfLoader::load("models/model.gltf", &asset_server).unwrap();
/// let entity = gltf_asset.spawn(commands, Transform::from_scale(Vec3::ONE * 10.0));
/// ```
/// This will spawn the model and return the parent entity ID.
#[derive(Asset, TypePath, Clone)]
pub struct GltfAsset {
    /// The path to the glTF file.
    pub path: String,
    /// The list of parsed glTF models. Each model is represented by a mesh and its associated material.
    /// Node transforms are baked directly into the mesh vertices.
    /// Format: (mesh_handle, material_handle)
    pub models: Vec<(Handle<Mesh>, Handle<PbrMaterial>)>,
}

/// Manager to load glTF models into the Bevy world.
/// 
/// # Methods
/// - `load`: Load a glTF file and register its models and materials into the Bevy world.
///   Returns a loaded `GltfAsset` which contains handles to the meshes and materials.
/// 
/// # Example
/// ```rust,no_run
/// let gltf_asset = GltfLoader::load("models/model.gltf", &asset_server).unwrap();
/// ```
pub struct GltfLoader;
impl GltfLoader {
    /// Load a glTF file and register its models and materials into the Bevy world.
    /// Returns a loaded `GltfAsset` which contains handles to the meshes and materials.
    /// 
    /// # Arguments
    /// - `path`: Path to the glTF file.
    /// - `asset_server`: Reference to the Bevy asset server for loading assets.
    /// 
    /// # Returns
    /// - `Result<GltfAsset, GltfError>`: On success, returns the loaded `GltfAsset`. On failure, returns a `GltfError`.
    pub fn load(
        path: &str,
        asset_server: &AssetServer
    ) -> Result<GltfAsset, GltfError> {
        debug!("Loading glTF model {}.", path);

        // Parse the glTF file
        let model = parser::parse_gltf(path)?;

        // Form and load the model into the Bevy world
        let (materials, meshes, bounding_boxes) = loader::form_models(&model)?;
        let gltf_asset = loader::load_models(&model.path, &materials, &meshes, &bounding_boxes, asset_server);
        Ok(gltf_asset)
    }
}
