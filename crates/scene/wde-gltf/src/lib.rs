use wde_logger::prelude::*;
use bevy::prelude::*;

mod model;
mod parser;
mod loader;
mod accessor;
mod error;

pub use error::GltfError;

#[derive(Asset, TypePath)]
pub struct GltfAsset {
    /// The path to the glTF file.
    pub path: String,
    /// The parsed glTF model.
    pub model: model::GltfModel,
}

/// Load a glTF file and spawn its content into the world.
/// Returns an error if the file cannot be loaded or parsed.
pub fn load_gltf(world: &mut World, path: &str) -> Result<(), GltfError> {
    info!("Loading glTF file: {}", path);

    // Parse the glTF file
    let model = parser::parse_gltf(path)?;
    // debug!("Parsed glTF model: {:#?}", model);

    // Load gltf into scene
    let (materials, meshes, bounding_boxes) = loader::form_models(&model)?;
    loader::load_models(world, &model.path, &materials, &meshes, &bounding_boxes);

    info!("Successfully loaded glTF file: {}", path);
    Ok(())
}
