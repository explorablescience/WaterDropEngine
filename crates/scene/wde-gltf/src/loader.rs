use bevy::prelude::*;
use wde_pbr::prelude::*;
use wde_renderer::prelude::*;

use crate::parser::{GltfAccessorComponentType, GltfModel};

pub fn load_models(world: &mut World, data: &[(Vec<u32>, Vec<Vertex>)]) {
    println!("Loading models into Bevy world...");

    // Add meshes to the asset server
    let mut meshes = world.resource_mut::<Assets<MeshAsset>>();
    let mut handles = Vec::new();
    for (i, (indices_data, vertices)) in data.iter().enumerate() {
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

    // Add a default material for the glTF models
    let material = world.resource_mut::<Assets<PbrMaterialAsset>>().add(PbrMaterialAsset {
        label: "gltf_material".to_string(),
        albedo: (1.0, 0.0, 0.0, 1.0),
        specular: 0.5,
        ..Default::default()
    });

    // Spawn entities with the meshes and material
    for (i, _) in data.iter().enumerate() {
        let mesh_handle = handles[i].clone();
        world.commands().spawn((
            Transform::from_xyz(0.0, 0.0, 0.0),
            Mesh(mesh_handle),
            PbrMaterial(material.clone()),
        ));
        println!("Spawned entity for mesh gltf_mesh_{}", i);
    }
}

pub fn form_meshes(model: &GltfModel) -> Vec<(Vec<u32>, Vec<Vertex>)> {
    println!("Forming meshes from glTF model...");

    // Iterate over meshes in the model
    let mut meshes_data: Vec<(Vec<u32>, Vec<Vertex>)> = Vec::new();
    for mesh in &model.meshes {
        // Iterate over primitives in the mesh
        for primitive in &mesh.primitives {
            // Get indices if they exist
            let indices = if let Some(index_accessor) = primitive.vertex_indexed {
                println!("Using index accessor: {}", index_accessor);
                
                // Retrieve index buffer data
                let data_type = &model.buffers[0].slices[index_accessor as usize].component_type;
                let buffer_slice = &model.buffers[0].slices[index_accessor as usize];
                let start = buffer_slice.byte_offset;
                let end = start + buffer_slice.byte_length;
                let index_data = &model.buffers[0].data[start..end];
                println!("Index data slice from {} to {}", start, end);

                // Parse indices based on component type
                match data_type {
                    GltfAccessorComponentType::UnsignedShort => {
                        Some(index_data
                            .chunks_exact(2)
                            .map(|b| u16::from_le_bytes([b[0], b[1]]) as u32)
                            .collect::<Vec<u32>>())
                    }
                    GltfAccessorComponentType::UnsignedInt => {
                        Some(index_data
                            .chunks_exact(4)
                            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                            .collect::<Vec<u32>>())
                    }
                    _ => {
                        println!("Unsupported index component type");
                        None
                    }
                }
            } else {
                None
            };
            println!("Indices: {:?}", indices);

            // Process vertex attributes
            let mut vertices: Vec<Vertex> = Vec::new();
            for (attr_name, accessor_index) in &primitive.vertex_attributes {
                println!("Attribute: {} with accessor index: {}", attr_name, accessor_index);

                let buffer_slice = &model.buffers[0].slices[*accessor_index as usize];
                let start = buffer_slice.byte_offset;
                let end = start + buffer_slice.byte_length;
                let attribute_data = &model.buffers[0].data[start..end];
                println!("Attribute data slice from {} to {}", start, end);
                
                let data: Vec<f32> = match buffer_slice.component_type {
                    GltfAccessorComponentType::Float => {
                        attribute_data
                            .chunks_exact(4)
                            .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                            .collect::<Vec<f32>>()
                    }
                    GltfAccessorComponentType::UnsignedShort => {
                        attribute_data
                            .chunks_exact(2)
                            .map(|b| u16::from_le_bytes([b[0], b[1]]) as f32)
                            .collect::<Vec<f32>>()
                    }
                    GltfAccessorComponentType::UnsignedInt => {
                        attribute_data
                            .chunks_exact(4)
                            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f32)
                            .collect::<Vec<f32>>()
                    }
                    _ => {
                        println!("Unsupported component type");
                        continue;
                    }
                };
                println!("Loaded {} floats for attribute {}", data.len(), attr_name);
                println!("Data: {:?}", data);

                // Assuming attribute is "POSITION" for simplicity
                if attr_name == "POSITION" {
                    for chunk in data.chunks_exact(3) {
                        vertices.push(Vertex {
                            position: chunk.try_into().unwrap(),
                            normal: [1.0, 0.0, 0.0], // Placeholder
                            uv: [0.0, 0.0],         // Placeholder
                        });
                    }
                } else {
                    println!("Unsupported attribute name: {}", attr_name);
                }
            }
            println!("Loaded {} vertices for primitive", vertices.len());
            println!("Vertices: {:?}", vertices);
            meshes_data.push((indices.unwrap_or(Vec::new()), vertices));
        }
    }
    meshes_data
}
