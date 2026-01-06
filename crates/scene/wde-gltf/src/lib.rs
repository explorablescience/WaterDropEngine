use bevy::prelude::*;

mod parser;
mod loader;

pub fn load_gltf(world: &mut World, path: &str) {
    println!("Loading glTF file from path: {}", path);


    // Parse the glTF file
    let model = parser::parse_gltf(path);
    println!("{:#?}", model);

    // Load gltf into scene
    let meshes_data = loader::form_meshes(&model);
    loader::load_models(world, &meshes_data);

    println!("Finished loading glTF file.");
}
