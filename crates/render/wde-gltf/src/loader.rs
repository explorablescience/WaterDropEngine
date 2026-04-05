use bevy::prelude::*;
use wde_logger::prelude::*;
use wde_pbr::prelude::*;
use wde_renderer::prelude::*;

use crate::GltfAsset;
use crate::accessor::{parse_attribute_as_f32, parse_indices};
use crate::error::GltfError;
use crate::material::GltfMaterial;
use crate::model::GltfModel;

/// A dataset representing mesh's data (indices, vertices), material ID.
type MeshDataSet = (Vec<u32>, Vec<Vertex>, usize);

/// Result type for form_models function
type FormedModelsResult =
    Result<(Vec<GltfMaterial>, Vec<MeshDataSet>, Vec<(Vec3, Vec3)>), GltfError>;

/// Register meshes and materials from the parsed glTF model into the Bevy world.
/// Returns a GltfAsset representing the loaded model.
pub fn load_models(
    folder_path: &str,
    raw_materials: &[GltfMaterial],
    raw_meshes: &[MeshDataSet],
    bounding_boxes: &[(Vec3, Vec3)],
    asset_server: &AssetServer
) -> GltfAsset {
    trace!("Loading {} models into Bevy world", raw_meshes.len());

    // Construct materials
    let materials_handles: Vec<Handle<PbrMaterial>> = raw_materials
        .iter()
        .map(|material| material.to_pbr(asset_server))
        .collect();

    // Add meshes to the asset server
    let mut models = Vec::new();
    for (i, (indices_data, vertices, material_id)) in raw_meshes.iter().enumerate() {
        let label = format!("gltf_mesh_{}", i);
        let (bb_min, bb_max) = bounding_boxes[i];
        let mesh_asset = Mesh {
            label: label.clone(),
            vertices: vertices.clone(),
            indices: indices_data.clone(),
            bbox: MeshBbox {
                min: bb_min,
                max: bb_max
            },
            use_ssbo: true
        };
        models.push((
            asset_server.add(mesh_asset),
            materials_handles[*material_id].clone()
        ));
        trace!(
            "Added mesh: {} (bbox min={:?}, max={:?})",
            label, bb_min, bb_max
        );
    }

    // Create a gltf asset to hold references to meshes and materials
    GltfAsset {
        path: folder_path.to_string(),
        models
    }
}

/// Transform a parsed `GltfModel` into engine `Vertex` arrays and indices.
pub fn form_models(model: &GltfModel) -> FormedModelsResult {
    trace!("Forming models from glTF data");

    // Iterate over meshes in the model
    let mut meshes_data: Vec<MeshDataSet> = Vec::new();
    let mut bounding_boxes: Vec<(Vec3, Vec3)> = Vec::new();
    let mut meshes_materials_ptr: Vec<usize> = Vec::new();
    let mut meshes_null_materials_ptr: Vec<usize> = Vec::new();
    for mesh in &model.meshes {
        // Iterate over primitives in the mesh
        for primitive in &mesh.primitives {
            // Get indices if they exist
            let indices = if let Some(accessor_data) = &primitive.vertex_indexed {
                let indices = parse_indices(accessor_data, &model.buffers[0])?;
                Some(indices)
            } else {
                None
            };

            // Process vertex attributes
            let mut vertices: Vec<Vertex> = Vec::new();
            let mut first = true;
            for (attr_name, accessor_data) in &primitive.vertex_attributes {
                let data: Vec<f32> = parse_attribute_as_f32(accessor_data, &model.buffers[0])?;

                // Populate vertices
                let vertex_count = accessor_data.count;
                if !first && vertices.len() != vertex_count {
                    return Err(GltfError::MismatchedVertexCount {
                        primitive: primitive.name.clone(),
                        expected: vertices.len(),
                        actual: vertex_count
                    });
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
                            vertices[i].tangent = [
                                data[i * 4],
                                data[i * 4 + 1],
                                data[i * 4 + 2],
                                data[i * 4 + 3]
                            ];
                        }
                        _ => {
                            warn!(
                                "Ignoring unsupported attribute '{}' in primitive {}",
                                attr_name, primitive.name
                            );
                        }
                    }
                }
            }
            trace!("Loaded {} vertices for {}", vertices.len(), primitive.name);

            // Apply node transform to vertices
            let transform_quat = Quat::from_array(primitive.rotation);
            let scale_vec = Vec3::from(primitive.scale);
            let translation_vec = Vec3::from(primitive.translation);
            for vertex in &mut vertices {
                // Transform position: translate -> rotate -> scale
                let pos = Vec3::from(vertex.position);
                let transformed_pos = transform_quat * (pos * scale_vec) + translation_vec;
                vertex.position = transformed_pos.into();

                // Transform normal: only rotate (scale is removed by normalization)
                let normal = Vec3::from(vertex.normal);
                let transformed_normal = (transform_quat * normal).normalize();
                vertex.normal = transformed_normal.into();

                // Transform tangent: only rotate the xyz components
                let tangent = Vec3::from([vertex.tangent[0], vertex.tangent[1], vertex.tangent[2]]);
                let transformed_tangent = (transform_quat * tangent).normalize();
                vertex.tangent[0] = transformed_tangent.x;
                vertex.tangent[1] = transformed_tangent.y;
                vertex.tangent[2] = transformed_tangent.z;
            }

            // Compute bounding box: try to use accessor min/max, otherwise compute from vertices
            let mut bb_min = None;
            let mut bb_max = None;
            for (attr_name, accessor_data) in &primitive.vertex_attributes {
                if attr_name == "POSITION" {
                    if let (Some(min), Some(max)) = (&accessor_data.min, &accessor_data.max)
                        && min.len() >= 3
                        && max.len() >= 3
                    {
                        // Transform the bounding box min/max
                        let min_vec = Vec3::new(min[0], min[1], min[2]);
                        let max_vec = Vec3::new(max[0], max[1], max[2]);
                        let transformed_min =
                            transform_quat * (min_vec * scale_vec) + translation_vec;
                        let transformed_max =
                            transform_quat * (max_vec * scale_vec) + translation_vec;
                        bb_min = Some(transformed_min.min(transformed_max));
                        bb_max = Some(transformed_min.max(transformed_max));
                    }
                    break;
                }
            }

            // If accessor didn't have min/max, compute from vertices
            let (final_min, final_max) = if let (Some(min), Some(max)) = (bb_min, bb_max) {
                (min, max)
            } else {
                let mut min = Vec3::new(f32::MAX, f32::MAX, f32::MAX);
                let mut max = Vec3::new(f32::MIN, f32::MIN, f32::MIN);
                for vertex in &vertices {
                    let pos = Vec3::from(vertex.position);
                    min = min.min(pos);
                    max = max.max(pos);
                }
                trace!("Computed bounding box from vertices for {}", primitive.name);
                (min, max)
            };

            let material_id = primitive.material_id.unwrap_or(0); // Default to first material if none assigned
            meshes_data.push((
                indices.unwrap_or(Vec::new()),
                vertices,
                material_id as usize
            ));
            bounding_boxes.push((final_min, final_max));

            // Store material pointer
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
    let mut material_datas = model.materials.clone();
    if !meshes_null_materials_ptr.is_empty() {
        warn!(
            "{} primitives without materials, using default white material",
            meshes_null_materials_ptr.len()
        );

        // Create a default material for primitives without assigned materials
        material_datas.push(GltfMaterial::default());

        // Assign default material to those meshes
        for &mesh_idx in &meshes_null_materials_ptr {
            meshes_materials_ptr[mesh_idx] = material_datas.len() - 1;
        }
    }
    Ok((material_datas, meshes_data, bounding_boxes))
}
