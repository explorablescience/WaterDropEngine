use bevy::prelude::*;

mod model;
mod parser;
mod loader;
mod accessor;

/// Load a glTF file and spawn its content into the world.
pub fn load_gltf(world: &mut World, path: &str) {
    println!("Loading glTF file from path: {}", path);


    // Parse the glTF file
    let model = parser::parse_gltf(path);
    println!("{:#?}", model);

    // Load gltf into scene
    let (materials, meshes) = loader::form_models(&model);
    loader::load_models(world, &model.path, &materials, &meshes);

    println!("Finished loading glTF file.");
}
