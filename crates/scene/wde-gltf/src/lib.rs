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
/// 
/// # Methods
/// - `spawn`: Spawn the glTF model into the Bevy world with a specified transform.
/// 
/// # Example
/// ```rust,no_run
/// let gltf_asset = GltfLoader::load("models/model.gltf", &asset_server).unwrap();
/// let entity = gltf_asset.spawn(commands, Transform::from_scale(Vec3::ONE * 10.0));
/// ```
/// This will spawn the model and return the parent entity ID.
#[derive(Asset, TypePath)]
pub struct GltfAsset {
    /// The path to the glTF file.
    pub path: String,
    /// The list of parsed glTF models. Each model is represented by a mesh and its associated material.
    pub models: Vec<(Handle<MeshAsset>, Handle<PbrMaterialAsset>)>,
}
impl GltfAsset {
    /// Util function to spawn the glTF model into the world.
    /// 
    /// # Arguments
    /// - `commands`: Mutable commands to spawn entities.
    /// - `transform`: The transform to apply to each spawned mesh.
    /// 
    /// # Returns
    /// - `Entity`: The parent entity ID containing the spawned model.
    pub fn spawn(&self, commands: &mut Commands, transform: Transform) -> Entity {
        trace!("Spawning glTF model from asset '{}'", self.path);

        // Create a parent entity to hold the model
        let parent = commands.spawn(Transform::default()).id();

        // Spawn the model's meshes and materials as children of the parent entity
        for (mesh_handle, material_handle) in &self.models {
            commands.entity(parent).with_children(|parent| {
                parent.spawn((
                    transform,
                    Mesh(mesh_handle.clone()),
                    PbrMaterial(material_handle.clone()),
                ));
            });
        }

        // Return the parent entity ID
        parent
    }
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
/// Then, you can spawn the model using `gltf_asset.spawn(commands, transform)`.
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
        info!("Loading glTF model from path '{}'.", path);

        // Parse the glTF file
        let model = parser::parse_gltf(path)?;

        // Form and load the model into the Bevy world
        let (materials, meshes, bounding_boxes) = loader::form_models(&model)?;
        let gltf_asset = loader::load_models(&model.path, &materials, &meshes, &bounding_boxes, asset_server);
        Ok(gltf_asset)
    }
}
