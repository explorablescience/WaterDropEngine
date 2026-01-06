use bevy::prelude::*;
use wde_pbr::prelude::*;
use wde_renderer::prelude::*;

use crate::accessor::{parse_attribute_as_f32, parse_indices};
use crate::model::GltfModel;

// A dataset representing mesh's data (positions, normals, uvs)
type MeshDataSet = (Vec<u32>, Vec<Vertex>);

// A dataset representing material's properties (base color, metallic, roughness)
type MaterialDataSet = ([f32; 4], Option<String>, f32, f32, Option<String>);

/// Register meshes and spawn entities from the provided datasets.
pub fn load_models(
    world: &mut World,
    folder_path: &str,
    raw_materials: &[MaterialDataSet],
    raw_meshes: &[MeshDataSet],
) {
    println!("Loading models into Bevy world...");

    // Construct materials
    let materials_handles: Vec<Handle<PbrMaterialAsset>> = raw_materials
        .iter()
        .map(
            |(base_color, base_color_texture, _metallic, roughness, metallic_roughness_texture)| {
                // Load base color texture if available
                let aldebo_texture_handle = base_color_texture
                    .as_ref()
                    .map(|texture_url| world.resource::<AssetServer>().load(format!("{}/{}", folder_path, texture_url)));

                // Load metallic-roughness texture if available
                let metallic_roughness_texture_handle = metallic_roughness_texture
                    .as_ref()
                    .map(|texture_url| world.resource::<AssetServer>().load(format!("{}/{}", folder_path, texture_url)));

                // Create and add the material to the asset server
                world
                    .resource_mut::<Assets<PbrMaterialAsset>>()
                    .add(PbrMaterialAsset {
                        label: "gltf_material".to_string(),
                        albedo: (base_color[0], base_color[1], base_color[2], base_color[3]),
                        specular: *roughness,
                        albedo_t: aldebo_texture_handle,
                        specular_t: metallic_roughness_texture_handle
                    })
            },
        )
        .collect();

    // Add meshes to the asset server
    let mut meshes = world.resource_mut::<Assets<MeshAsset>>();
    let mut handles = Vec::new();
    for (i, (indices_data, vertices)) in raw_meshes.iter().enumerate() {
        let label = format!("gltf_mesh_{}", i);
        let mesh_asset = MeshAsset {
            label: label.clone(),
            vertices: vertices.clone(),
            indices: indices_data.clone(),
            bounding_box: ModelBoundingBox::default(), // Placeholder
        };
        let handle = meshes.add(mesh_asset);
        handles.push(handle);
        println!("Added mesh asset: {}", label);
    }

    // Spawn entities with the meshes and material
    for (i, _) in raw_meshes.iter().enumerate() {
        world.commands().spawn((
            Transform::from_xyz(0.0, 0.0, 0.0),
            Mesh(handles[i].clone()),
            PbrMaterial(materials_handles[i].clone()),
        ));
        println!("Spawned entity for mesh {}", i);
    }
}

/// Transform a parsed `GltfModel` into engine `Vertex` arrays and indices.
pub fn form_models(model: &GltfModel) -> (Vec<MaterialDataSet>, Vec<MeshDataSet>) {
    println!("Forming models from glTF data...");

    // Extract materials data
    println!("Forming materials from glTF data...");
    let mut material_datas: Vec<MaterialDataSet> = Vec::new();
    for material in &model.materials {
        println!("Processing material: {:?}", material);
        material_datas.push((
            material.base_color_factor,
            material.base_color_texture_url.clone(),
            material.metallic_factor,
            material.roughness_factor,
            material.metallic_roughness_texture_url.clone(),
        ));
    }

    // Iterate over meshes in the model
    println!("Forming meshes from glTF model...");
    let mut meshes_data: Vec<MeshDataSet> = Vec::new();
    let mut meshes_materials_ptr: Vec<usize> = Vec::new();
    let mut meshes_null_materials_ptr: Vec<usize> = Vec::new();
    for mesh in &model.meshes {
        // Iterate over primitives in the mesh
        for primitive in &mesh.primitives {
            // Get indices if they exist
            let indices = if let Some(accessor_data) = &primitive.vertex_indexed {
                println!("Processing indices with accessor data: {:?}", accessor_data);
                let indices = parse_indices(accessor_data, &model.buffers[0]);
                println!("Loaded {} indices", indices.len());
                Some(indices)
            } else {
                None
            };

            // Process vertex attributes
            let mut vertices: Vec<Vertex> = Vec::new();
            let mut first = true;
            for (attr_name, accessor_data) in &primitive.vertex_attributes {
                println!(
                    "Attribute: {} with accessor data: {:?}",
                    attr_name, accessor_data
                );

                let data: Vec<f32> = parse_attribute_as_f32(accessor_data, &model.buffers[0]);
                println!("Loaded {} floats for attribute {}", data.len(), attr_name);

                // Populate vertices
                let vertex_count = accessor_data.count;
                if !first && vertices.len() != vertex_count {
                    panic!("Mismatched vertex counts across attributes");
                }
                if first {
                    vertices.resize(vertex_count, Vertex::default());
                    first = false;
                }
                for i in 0..vertex_count {
                    match attr_name.as_str() {
                        "POSITION" => {
                            vertices[i].position = [data[i * 3], data[i * 3 + 1], data[i * 3 + 2]];
                        }
                        "NORMAL" => {
                            vertices[i].normal = [data[i * 3], data[i * 3 + 1], data[i * 3 + 2]];
                        }
                        "TEXCOORD_0" => {
                            vertices[i].uv = [data[i * 2], data[i * 2 + 1]];
                        }
                        "TANGENT" => {
                            continue;
                        }
                        _ => {
                            panic!(
                                "Unsupported attribute name during vertex population: {}",
                                attr_name
                            );
                        }
                    }
                }
            }
            println!("Loaded {} vertices for primitive", vertices.len());
            meshes_data.push((indices.unwrap_or(Vec::new()), vertices));

            // Store material pointer
            println!(
                "Associating material for primitive: {:?}",
                primitive.material_id
            );
            match primitive.material_id {
                Some(mat_id) => meshes_materials_ptr.push(mat_id as usize),
                None => {
                    meshes_materials_ptr.push(usize::MAX); // Indicate missing material
                    meshes_null_materials_ptr.push(meshes_data.len() - 1);
                }
            }
        }
    }

    // Log materials without assigned primitives
    if !meshes_null_materials_ptr.is_empty() {
        println!("The following meshes have primitives without assigned materials");

        // Create a default material for primitives without assigned materials
        let default_material = ([1.0, 1.0, 1.0, 1.0], None, 0.0, 1.0, None);
        material_datas.push(default_material);

        // Assign default material to those meshes
        for &mesh_idx in &meshes_null_materials_ptr {
            println!("  Mesh at index {} will use default material", mesh_idx);
            meshes_materials_ptr[mesh_idx] = material_datas.len() - 1;
        }
    }
    (material_datas, meshes_data)
}
