use std::collections::HashMap;
use wde_logger::prelude::*;

use base64::{Engine, engine::general_purpose};
use serde_json::Value;
use crate::error::GltfError;
use crate::material::GltfMaterial;
use crate::model::{
    AccessorData, BufferSliceData, GltfAccessorComponentType, GltfBuffer, GltfMesh, GltfModel, MeshPrimitive
};

/// Parse a glTF 2.0 JSON file from `res/` and build an in-memory `GltfModel`.
pub fn parse_gltf(path: &str) -> Result<GltfModel, GltfError> {
    // Extract filename and folder path
    let std_path = std::path::Path::new(path);
    let filename = std_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unnamed.gltf")
        .to_string();
    let folder_path = std_path
        .parent()
        .and_then(|p| p.to_str())
        .unwrap_or("")
        .to_string();
    trace!("Parsing glTF file '{}'.", path);

    // Read the file content and parse JSON
    let content = std::fs::read_to_string(format!("./res/{}", path))?;
    let json: Value = serde_json::from_str(&content)?;

    // Assert asset version
    let asset = &json["asset"];
    let version = asset["version"]
        .as_str()
        .ok_or(GltfError::MissingField("asset.version".to_string()))?;
    if version != "2.0" {
        return Err(GltfError::UnsupportedVersion(version.to_string()));
    }

    // Read buffers
    let buffers = json["buffers"]
        .as_array()
        .ok_or(GltfError::MissingField("buffers".to_string()))?;
    if buffers.is_empty() {
        return Err(GltfError::NoBuffers);
    } else if buffers.len() > 1 {
        return Err(GltfError::MultipleBuffers(buffers.len()));
    }
    let single_buffer = &buffers[0];
    let buffer_uri = single_buffer["uri"]
        .as_str()
        .ok_or(GltfError::MissingField("buffer.uri".to_string()))?;

    // Decode buffer data
    let buffer_data: Vec<u8> = if buffer_uri.starts_with("data:application/octet-stream;base64,") {
        let base64_data = buffer_uri
            .strip_prefix("data:application/octet-stream;base64,")
            .ok_or(GltfError::Base64Error("Invalid data URI".to_string()))?;
        general_purpose::STANDARD.decode(base64_data)?
    } else {
        let buffer_path = std::path::Path::new(&folder_path).join(buffer_uri);
        std::fs::read(format!("./res/{}", buffer_path.display()))?
    };

    // Read which accessors correspond to which buffer views
    let accessors = json["accessors"]
        .as_array()
        .ok_or(GltfError::MissingField("accessors".to_string()))?;

    // Read buffer views
    let buffer_views = json["bufferViews"]
        .as_array()
        .ok_or(GltfError::MissingField("bufferViews".to_string()))?;
    trace!("Found {} bufferViews and {} accessors", buffer_views.len(), accessors.len());
    
    let mut slices_data = Vec::new();
    for (i, buffer_view) in buffer_views.iter().enumerate() {
        // For simplicity, we only handle buffer index 0 in this example
        let buffer_index = buffer_view["buffer"]
            .as_i64()
            .ok_or(GltfError::MissingField(format!("bufferView[{}].buffer", i)))?;
        if buffer_index != 0 {
            return Err(GltfError::MultipleBuffers(buffer_index as usize + 1));
        }

        // Get byte offset and length for this view
        let byte_offset = buffer_view
            .get("byteOffset")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let _byte_length = buffer_view["byteLength"]
            .as_i64()
            .ok_or(GltfError::MissingField(format!("bufferView[{}].byteLength", i)))?;

        // Store buffer slice data
        let slice_data = BufferSliceData {
            byte_offset: byte_offset as usize
        };
        slices_data.push(slice_data);
    }
    let buffer = GltfBuffer {
        data: buffer_data,
        slices: slices_data,
    };

    // Retrieve the nodes of the default scene
    let scene_index = json["scene"]
        .as_i64()
        .ok_or(GltfError::MissingField("scene".to_string()))?;
    let nodes_list = json["scenes"][scene_index as usize]["nodes"]
        .as_array()
        .ok_or(GltfError::MissingField("scenes.nodes".to_string()))?;
    trace!("Processing {} nodes", nodes_list.len());

    // Handle each nodes
    let mut mesh_primitives: Vec<MeshPrimitive> = Vec::new();
    let mut material_datas: Vec<GltfMaterial> = Vec::new();
    let mut material_map: HashMap<i64, usize> = HashMap::new();
    for node in nodes_list {
        // Retrieve node data
        let node_idx = node.as_i64()
            .ok_or(GltfError::JsonError("Invalid node index".to_string()))?;
        let node = &json["nodes"][node_idx as usize];

        // Handle meshes in the node
        let mesh_index = node["mesh"].as_i64()
            .ok_or(GltfError::MissingField("node.mesh".to_string()))?;
        let mesh = &json["meshes"][mesh_index as usize];

        // Process primitives in the mesh (each primitive contains a description of how to draw a part of the mesh)
        let primitives = mesh["primitives"]
            .as_array()
            .ok_or(GltfError::MissingField("mesh.primitives".to_string()))?;
        let name = mesh
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("unnamed_mesh");
        for (i, primitive) in primitives.iter().enumerate() {
            // Process optional indexed vertices
            let vertex_indexed_accessor_id = primitive
                .get("indices")
                .and_then(|index_accessor| index_accessor.as_i64());
            let vertex_indexed_accessor = if let Some(index_accessor) = vertex_indexed_accessor_id {
                let accessor = &accessors[index_accessor as usize];

                // Build AccessorData
                let buffer_view_index = accessor["bufferView"]
                    .as_i64()
                    .ok_or(GltfError::MissingField("accessor.bufferView".to_string()))?;
                let byte_offset = accessor
                    .get("byteOffset")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as usize;
                let count = accessor["count"]
                    .as_i64()
                    .ok_or(GltfError::MissingField("accessor.count".to_string()))? as usize;
                let component_type_value = accessor["componentType"]
                    .as_i64()
                    .ok_or(GltfError::MissingField("accessor.componentType".to_string()))?;
                let component_type = GltfAccessorComponentType::from_i64(component_type_value)
                    .ok_or(GltfError::UnsupportedComponentType(component_type_value))?;
                let min = accessor.get("min").and_then(|v| {
                    v.as_array().map(|arr| {
                        arr.iter().filter_map(|val| val.as_f64().map(|f| f as f32)).collect()
                    })
                });
                let max = accessor.get("max").and_then(|v| {
                    v.as_array().map(|arr| {
                        arr.iter().filter_map(|val| val.as_f64().map(|f| f as f32)).collect()
                    })
                });
                Some(AccessorData {
                    buffer_view_index,
                    byte_offset,
                    component_type,
                    count,
                    accessor_type: accessor["type"]
                        .as_str()
                        .expect("Accessor missing type")
                        .to_string(),
                    min,
                    max,
                })
            } else {
                None
            };

            // Process vertex attributes
            let attributes = &primitive["attributes"];
            let mut vertex_attributes: Vec<(String, AccessorData)> = Vec::new();
            for (attr_name, accessor_value) in attributes.as_object().expect("Attributes is not an object") {
                // Retrieve accessor index
                let accessor_index = accessor_value
                    .as_i64()
                    .expect("Invalid accessor index for attribute");
                let accessor = &accessors[accessor_index as usize];

                // Build AccessorData
                let buffer_view_index = accessor["bufferView"]
                    .as_i64()
                    .expect("Accessor missing bufferView");
                let byte_offset = accessor
                    .get("byteOffset")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as usize;
                let count = accessor["count"]
                    .as_i64()
                    .expect("Accessor missing count") as usize;
                let component_type_value = accessor["componentType"]
                    .as_i64()
                    .expect("Accessor missing componentType");
                let component_type = GltfAccessorComponentType::from_i64(component_type_value)
                    .expect("Unsupported accessor component type");
                let accessor_type = accessor["type"]
                    .as_str()
                    .expect("Accessor missing type")
                    .to_string();
                let min = accessor.get("min").and_then(|v| {
                    v.as_array().map(|arr| {
                        arr.iter().filter_map(|val| val.as_f64().map(|f| f as f32)).collect()
                    })
                });
                let max = accessor.get("max").and_then(|v| {
                    v.as_array().map(|arr| {
                        arr.iter().filter_map(|val| val.as_f64().map(|f| f as f32)).collect()
                    })
                });
                let accessor_data = AccessorData {
                    buffer_view_index,
                    byte_offset,
                    component_type,
                    count,
                    accessor_type,
                    min,
                    max,
                };
                vertex_attributes.push((attr_name.clone(), accessor_data));
            }

            // Process material (if any)
            let material_index = primitive
                .get("material")
                .and_then(|v| v.as_i64());
            if let Some(mat_index) = material_index
                && !material_map.contains_key(&mat_index)
            {
                let mat_data = parse_material(Some(mat_index), &json["materials"][mat_index as usize], &json, &folder_path)?;
                if let Some(mat) = mat_data {
                    material_map.insert(mat_index, material_datas.len());
                    material_datas.push(mat);
                }
            }

            // Extract node transform
            let translation = node.get("translation")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    [
                        arr.first().and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                        arr.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                        arr.get(2).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                    ]
                })
                .unwrap_or([0.0, 0.0, 0.0]);
            
            let rotation = node.get("rotation")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    [
                        arr.first().and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                        arr.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                        arr.get(2).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                        arr.get(3).and_then(|v| v.as_f64()).unwrap_or(1.0) as f32,
                    ]
                })
                .unwrap_or([0.0, 0.0, 0.0, 1.0]);
            
            let scale = node.get("scale")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    [
                        arr.first().and_then(|v| v.as_f64()).unwrap_or(1.0) as f32,
                        arr.get(1).and_then(|v| v.as_f64()).unwrap_or(1.0) as f32,
                        arr.get(2).and_then(|v| v.as_f64()).unwrap_or(1.0) as f32,
                    ]
                })
                .unwrap_or([1.0, 1.0, 1.0]);

            // Build MeshPrimitive
            let mesh_primitive = MeshPrimitive {
                name: format!("{}_primitive_{}", name, i),
                vertex_indexed: vertex_indexed_accessor,
                vertex_attributes,
                material_id: material_index.map(|idx| idx as u32),
                translation,
                rotation,
                scale,
            };
            mesh_primitives.push(mesh_primitive);
        }
    }

    // Add the remaining materials that were not assigned to any mesh
    let raw_materials = json["materials"]
        .as_array()
        .ok_or(GltfError::MissingField("materials".to_string()))?;
    for (i, material_json) in raw_materials.iter().enumerate() {
        if !material_map.contains_key(&(i as i64)) {
            let mat_data = parse_material(Some(i as i64), material_json, &json, &folder_path)?;
            if let Some(mat) = mat_data {
                material_datas.push(mat);
            }
        }
    }

    // GltfModel construction
    Ok(GltfModel {
        filename,
        path: folder_path,
        buffers: vec![buffer],
        meshes: vec![GltfMesh {
            primitives: mesh_primitives,
        }],
        materials: material_datas,
    })
}


/// Parse material data from the glTF JSON
fn parse_material(material_index: Option<i64>, material_json: &Value, json: &Value, folder_path: &str) -> Result<Option<GltfMaterial>, GltfError> {
    if material_index.is_none() {
        return Ok(None);
    }
    let pbr = &material_json["pbrMetallicRoughness"];

    // Extract material number properties
    let base_color_factor = pbr["baseColorFactor"]
        .as_array()
        .unwrap_or(&vec![Value::from(1.0), Value::from(1.0), Value::from(1.0), Value::from(1.0)])
        .iter()
        .map(|v| v.as_f64().unwrap_or(1.0) as f32)
        .collect::<Vec<f32>>();
    let metallic_factor = pbr["metallicFactor"]
        .as_f64()
        .unwrap_or(1.0) as f32;
    let roughness_factor = pbr["roughnessFactor"]
        .as_f64()
        .unwrap_or(1.0) as f32;

    // Extract material textures
    const OPT_TEX_CANDIDATES: [(&str, &str); 4] = [
        ("pbr", "baseColorTexture"),
        ("pbr", "metallicRoughnessTexture"),
        ("root", "normalTexture"),
        ("root", "occlusionTexture")
    ];
    let mut opt_textures_urls = HashMap::new();
    for tex_name in OPT_TEX_CANDIDATES {
        let try_get_image = match tex_name.0 {
            "pbr" => pbr.get(tex_name.1),
            "root" => material_json.get(tex_name.1),
            _ => None
        };
        if let Some(tex_info) = try_get_image {
            let tex_id = tex_info["index"].as_i64();

            // Retrieve texture data
            if let Some(tex_id) = tex_id {
                let texture = &json["textures"][tex_id as usize];
                let image_index = texture["source"]
                    .as_i64()
                    .ok_or(GltfError::MissingField("texture.source".to_string()))?;

                // Retrieve image URI
                let image = &json["images"][image_index as usize];
                let image_uri = image["uri"]
                    .as_str()
                    .ok_or(GltfError::MissingField("image.uri".to_string()))?
                    .to_string();
                opt_textures_urls.insert(tex_name.1.to_string(), image_uri);
            }
        }
    }

    // Build GltfMaterial
    Ok(Some(GltfMaterial {
        name: material_json
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(&format!("material_{}", material_index.unwrap()))
            .to_string(),
        folder_path: folder_path.to_string(),
        base_color: base_color_factor.try_into().unwrap_or([1.0, 1.0, 1.0, 1.0]),
        metallic: metallic_factor,
        roughness: roughness_factor,
        base_color_tex_url: opt_textures_urls.get("baseColorTexture").cloned(),
        metallic_roughness_tex_url: opt_textures_urls.get("metallicRoughnessTexture").cloned(),
        normal_tex_url: opt_textures_urls.get("normalTexture").cloned(),
        occlusion_tex_url: opt_textures_urls.get("occlusionTexture").cloned(),
    }))
}
