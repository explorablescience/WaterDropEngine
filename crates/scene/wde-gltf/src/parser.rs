use std::collections::HashMap;

use base64::{Engine, engine::general_purpose};
use serde_json::Value;
use crate::model::{
    AccessorData, BufferSliceData, GltfAccessorComponentType, GltfBuffer, GltfMesh, GltfModel, MaterialData, MeshPrimitive
};

/// Parse a glTF 2.0 JSON file from `res/` and build an in-memory `GltfModel`.
pub fn parse_gltf(path: &str) -> GltfModel {
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
    println!("Parsing glTF file: {}", filename);
    println!("  Folder path: {}", folder_path);

    // Read the file content
    let content =
        std::fs::read_to_string(format!("./res/{}", path)).expect("Failed to read glTF file");

    // Parse to JSON structure
    let json: Value = serde_json::from_str(&content).expect("Failed to parse glTF JSON");
    println!("Successfully parsed glTF JSON");

    // Assert asset version
    let asset = &json["asset"];
    let version = asset["version"]
        .as_str()
        .expect("Missing 'version' field in asset");
    if version != "2.0" {
        panic!("Unsupported glTF version: {}", version);
    }
    println!("  glTF asset version: {}", version);

    // Read buffers
    let buffers = &json["buffers"]
        .as_array()
        .expect("No buffers found in glTF JSON");
    if buffers.is_empty() {
        panic!("No buffers found in glTF JSON");
    } else if buffers.len() > 1 {
        println!("Multiple buffers found in glTF JSON:");
    }
    let single_buffer = &buffers[0];
    let buffer_uri = single_buffer["uri"]
        .as_str()
        .expect("Buffer URI is missing");
    println!("Using buffer URI: {}", buffer_uri);

    // Decode buffer data
    let buffer_data: Vec<u8> = if buffer_uri.starts_with("data:application/octet-stream;base64,") {
        let base64_data = buffer_uri
            .strip_prefix("data:application/octet-stream;base64,")
            .expect("Failed to strip data URI prefix");
        general_purpose::STANDARD
            .decode(base64_data)
            .expect("Failed to decode base64 buffer data")
    } else {
        let buffer_path = std::path::Path::new(&folder_path).join(buffer_uri);
        println!("  Resolved buffer file path: {:?}", buffer_path);
        std::fs::read(format!("./res/{}", buffer_path.display()))
            .expect("Failed to read buffer file from path")
    };

    // Read which accessors correspond to which buffer views
    let accessors = &json["accessors"]
        .as_array()
        .expect("No accessors found in glTF JSON");

    // Read buffer views
    let buffer_views = &json["bufferViews"]
        .as_array()
        .expect("No bufferViews found in glTF JSON");
    println!(
        "Found {} bufferViews and {} accessors",
        buffer_views.len(),
        accessors.len()
    );
    let mut slices_data = Vec::new();
    for (i, buffer_view) in buffer_views.iter().enumerate() {
        // For simplicity, we only handle buffer index 0 in this example
        let buffer_index = buffer_view["buffer"]
            .as_i64()
            .expect("Buffer index missing in bufferView");
        if buffer_index != 0 {
            panic!("Only single buffer (index 0) is supported in this loader");
        }

        // Get byte offset and length for this view
        let byte_offset = buffer_view
            .get("byteOffset")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let byte_length = buffer_view["byteLength"]
            .as_i64()
            .expect("byteLength missing in bufferView");
        println!(
            "  BufferView {}: buffer index {}, byte offset {}, byte length {}",
            i, buffer_index, byte_offset, byte_length
        );

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
    println!("Processed {} buffer slices", buffer.slices.len());
    println!("===> Buffer slices: {:?}", buffer.slices);

    // Retrieve the nodes of the default scene
    let scene_index = json["scene"]
        .as_i64()
        .expect("Missing 'scene' field in glTF JSON");
    let nodes_list = json["scenes"][scene_index as usize]["nodes"]
        .as_array()
        .expect("Scene index not found, or 'nodes' field is missing");
    println!("Found {} nodes in the scene", nodes_list.len());

    // Handle each nodes
    let mut mesh_primitives: Vec<MeshPrimitive> = Vec::new();
    let mut material_datas: Vec<MaterialData> = Vec::new();
    let mut material_map: HashMap<i64, usize> = HashMap::new();
    for node in nodes_list {
        // Retrieve node data
        let node = &json["nodes"][node.as_i64().unwrap() as usize];
        println!("  Processing node: {:?}", node);

        // Handle meshes in the node
        let mesh_index = node["mesh"].as_i64().expect("Node does not contain a mesh");
        let mesh = &json["meshes"][mesh_index as usize];
        println!("Processing mesh: {:?}", mesh);

        // Process primitives in the mesh (each primitive contains a description of how to draw a part of the mesh)
        let primitives = mesh["primitives"]
            .as_array()
            .expect("Mesh does not contain primitives");
        let name = mesh
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("unnamed_mesh");
        for (i, primitive) in primitives.iter().enumerate() {
            // println!("Processing primitive {}: {}", i, name);

            // Process optional indexed vertices
            let vertex_indexed_accessor_id = primitive
                .get("indices")
                .map(|index_accessor| index_accessor.as_i64().expect("Invalid indices accessor"));
            let vertex_indexed_accessor = if let Some(index_accessor) = vertex_indexed_accessor_id {
                let accessor = &accessors[index_accessor as usize];
                // println!("  Indexed vertices accessor: {:?}", accessor);

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
                Some(AccessorData {
                    buffer_view_index,
                    byte_offset,
                    component_type,
                    count,
                    accessor_type: accessor["type"]
                        .as_str()
                        .expect("Accessor missing type")
                        .to_string(),
                })
            } else {
                None
            };
            // println!("  Indexed vertices accessor: {:?}", vertex_indexed_accessor);

            // Process vertex attributes
            let attributes = &primitive["attributes"];
            let mut vertex_attributes: Vec<(String, AccessorData)> = Vec::new();
            for (attr_name, accessor_value) in attributes.as_object().expect("Attributes is not an object") {
                // Retrieve accessor index
                let accessor_index = accessor_value
                    .as_i64()
                    .expect("Invalid accessor index for attribute");
                let accessor = &accessors[accessor_index as usize];
                // println!("  Attribute '{}' uses accessor: {:?}", attr_name, accessor);

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
                let accessor_data = AccessorData {
                    buffer_view_index,
                    byte_offset,
                    component_type,
                    count,
                    accessor_type,
                };
                vertex_attributes.push((attr_name.clone(), accessor_data));
            }

            // Process material (if any)
            let material_index = primitive
                .get("material")
                .and_then(|v| v.as_i64());
            let material_data = if let Some(mat_index) = material_index {
                if let Some(&mapped_index) = material_map.get(&mat_index) {
                    Some(material_datas[mapped_index].clone())
                } else {
                    let mat_data = parse_material(Some(mat_index), &json["materials"][mat_index as usize], &json);
                    if let Some(ref mat) = mat_data {
                        material_datas.push(mat.clone());
                        material_map.insert(mat_index, material_datas.len() - 1);
                    }
                    mat_data
                }
            } else {
                None
            };
            println!("  Material data: {:?}", material_data);

            // Build MeshPrimitive
            let mesh_primitive = MeshPrimitive {
                name: format!("{}_primitive_{}", name, i),
                vertex_indexed: vertex_indexed_accessor,
                vertex_attributes,
                material_id: material_index.map(|idx| idx as u32)
            };
            println!("  Processed primitive: {:?}", mesh_primitive.clone());
            mesh_primitives.push(mesh_primitive);
        }
        println!("Processed {} primitives in the mesh", mesh_primitives.len());
        println!("===> All primitives: {:?}", mesh_primitives);
    }

    // GltfModel construction
    GltfModel {
        filename,
        path: folder_path,
        buffers: vec![buffer],
        meshes: vec![GltfMesh {
            primitives: mesh_primitives,
        }],
        materials: material_datas,
    }
}


/// Parse material data from the glTF JSON
fn parse_material(material_index: Option<i64>, material_json: &Value, json: &Value) -> Option<MaterialData> {
    material_index?;
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
    let base_color_texture_index = if let Some(tex_info) = pbr.get("baseColorTexture") {
        let tex_id = tex_info["index"].as_i64();

        // Retrieve texture data
        if let Some(tex_id) = tex_id {
            let texture = &json["textures"][tex_id as usize];
            let image_index = texture["source"]
                .as_i64()
                .expect("Texture missing source image index");
            Some(image_index)
        } else {
            None
        }
    } else {
        None
    };
    let metallic_roughness_texture_index = if let Some(tex_info) = pbr.get("metallicRoughnessTexture") {
        let tex_id = tex_info["index"].as_i64();

        // Retrieve texture data
        if let Some(tex_id) = tex_id {
            let texture = &json["textures"][tex_id as usize];
            let image_index = texture["source"]
                .as_i64()
                .expect("Texture missing source image index");
            Some(image_index)
        } else {
            None
        }
    } else {
        None
    };

    Some(MaterialData {
        name: material_json
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(&format!("material_{}", material_index.unwrap()))
            .to_string(),
        base_color_factor: [
            base_color_factor.first().cloned().unwrap_or(1.0),
            base_color_factor.get(1).cloned().unwrap_or(1.0),
            base_color_factor.get(2).cloned().unwrap_or(1.0),
            base_color_factor.get(3).cloned().unwrap_or(1.0),
        ],
        metallic_factor,
        roughness_factor,
        base_color_texture_url: base_color_texture_index.map(|img_index| {
            let image = &json["images"][img_index as usize];
            image["uri"]
                .as_str()
                .expect("Image missing URI")
                .to_string()
        }),
        metallic_roughness_texture_url: metallic_roughness_texture_index.map(|img_index| {
            let image = &json["images"][img_index as usize];
            image["uri"]
                .as_str()
                .expect("Image missing URI")
                .to_string()
        })
    })
}
