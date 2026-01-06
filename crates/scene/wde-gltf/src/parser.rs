use base64::{Engine, engine::general_purpose};
use bevy::platform::collections::HashMap;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct GltfModel {
    pub buffers: Vec<Buffer>,
    pub meshes: Vec<GltfMesh>,
}

#[derive(Debug, Clone)]
pub struct GltfMesh {
    pub primitives: Vec<MeshPrimitive>,
}

#[derive(Debug, Clone)]
pub struct MeshPrimitive {
    /// Whether the vertices are indexed. If so, which index accessor to use
    pub vertex_indexed: Option<i64>,
    /// List of attribute names of vertices along with their accessor indices
    pub vertex_attributes: Vec<(String, i64)>,
}

#[derive(Debug, Clone)]
pub struct Buffer {
    pub data: Vec<u8>,
    pub slices: Vec<BufferSliceData>,
}

#[derive(Debug, Clone)]
pub struct BufferSliceData {
    pub byte_offset: usize,
    pub byte_length: usize,
    pub accessor_type: String,
    pub component_type: GltfAccessorComponentType,
    pub count: usize,
}

#[derive(Debug, Clone, Copy)]
pub enum GltfAccessorComponentType {
    Byte = 5120,
    UnsignedByte = 5121,
    Short = 5122,
    UnsignedShort = 5123,
    UnsignedInt = 5125,
    Float = 5126,
}
impl GltfAccessorComponentType {
    pub fn from_i64(value: i64) -> Option<Self> {
        match value {
            5120 => Some(GltfAccessorComponentType::Byte),
            5121 => Some(GltfAccessorComponentType::UnsignedByte),
            5122 => Some(GltfAccessorComponentType::Short),
            5123 => Some(GltfAccessorComponentType::UnsignedShort),
            5125 => Some(GltfAccessorComponentType::UnsignedInt),
            5126 => Some(GltfAccessorComponentType::Float),
            _ => None,
        }
    }
}

pub fn parse_gltf(path: &str) -> GltfModel {
    // Read the file content
    let content = std::fs::read_to_string(format!("./res/{}", path)).expect("Failed to read glTF file");

    // Parse to JSON structure
    let json: Value = serde_json::from_str(&content).expect("Failed to parse glTF JSON");
    println!("Successfully parsed glTF JSON");

    // Assert asset version
    let asset = &json["asset"];
    let version = asset["version"].as_str().expect("Missing 'version' field in asset");
    if version != "2.0" {
        panic!("Unsupported glTF version: {}", version);
    }
    println!("  glTF asset version: {}", version);



    // Read buffers
    let buffers = &json["buffers"].as_array().expect("No buffers found in glTF JSON");
    if buffers.is_empty() {
        panic!("No buffers found in glTF JSON");
    } else if buffers.len() > 1 {
        println!("Multiple buffers found in glTF JSON:");
    }
    let single_buffer = &buffers[0];
    let buffer_uri = single_buffer["uri"].as_str().expect("Buffer URI is missing");
    println!("Using buffer URI: {}", buffer_uri);
    let base64_data = buffer_uri
        .strip_prefix("data:application/octet-stream;base64,")
        .ok_or("Invalid data URI format").expect("Failed to strip data URI prefix");
    let buffer_data = general_purpose::STANDARD
        .decode(base64_data)
        .expect("Failed to decode base64 buffer data");
    println!("  Decoded buffer data length: {}", buffer_data.len());
    println!("  Buffer data (first 64 bytes): {:?}", &buffer_data[..64.min(buffer_data.len())]);

    // Read which accessors correspond to which buffer views
    let mut accessors: HashMap<Value, _> = HashMap::new();
    let raw_accessors = &json["accessors"].as_array().expect("No accessors found in glTF JSON");
    for accessor in raw_accessors.iter() {
        let buffer_view_index = accessor["bufferView"].as_i64().expect("bufferView missing in accessor");
        accessors.insert(buffer_view_index.into(), accessor);
    }

    // Read buffer views
    let buffer_views = &json["bufferViews"].as_array().expect("No bufferViews found in glTF JSON");
    println!("Found {} bufferViews and {} accessors", buffer_views.len(), accessors.len());
    let mut slices_data = Vec::new();
    for (i, buffer_view) in buffer_views.iter().enumerate() {
        // For simplicity, we only handle buffer index 0 in this example
        let buffer_index = buffer_view["buffer"].as_i64().expect("Buffer index missing in bufferView");
        if buffer_index != 0 {
            panic!("Only single buffer (index 0) is supported in this loader");
        }

        // Get byte offset and length for this view
        let byte_offset = buffer_view.get("byteOffset").and_then(|v| v.as_i64()).unwrap_or(0);
        let byte_length = buffer_view["byteLength"].as_i64().expect("byteLength missing in bufferView");
        println!("  BufferView {}: buffer index {}, byte offset {}, byte length {}", i, buffer_index, byte_offset, byte_length);

        // Find associated accessors
        if !accessors.contains_key(&Value::from(i as i64)) {
            panic!("No accessor found for bufferView {}", i);
        }
        let accessor = accessors.get(&Value::from(i as i64)).unwrap();
        let accessor_type = accessor["type"].as_str().expect("Accessor type missing");
        let component_type = GltfAccessorComponentType::from_i64(
            accessor["componentType"].as_i64().expect("Accessor componentType missing")
        ).expect("Unknown accessor componentType");
        let count = accessor["count"].as_i64().expect("Accessor count missing");
        let max_values = accessor.get("max").and_then(|v| v.as_array()).expect("Accessor max missing");
        let min_values = accessor.get("min").and_then(|v| v.as_array()).expect("Accessor min missing");
        println!("    Associated accessor: type {}, componentType {:?}, count {}, max {:?}, min {:?}", accessor_type, component_type, count, max_values, min_values);
    
        // Store buffer slice data
        let slice_data = BufferSliceData {
            byte_offset: byte_offset as usize,
            byte_length: byte_length as usize,
            accessor_type: accessor_type.to_string(),
            component_type,
            count: count as usize,
        };
        slices_data.push(slice_data);
    }
    let buffer = Buffer {
        data: buffer_data,
        slices: slices_data,
    };
    println!("Processed {} buffer slices", buffer.slices.len());
    println!("===> Buffer details: {:?}", buffer);



    // Retrieve the nodes of the default scene
    let scene_index = &json["scene"].as_i64().expect("Missing 'scene' field in glTF JSON");
    let nodes_list = &json["scenes"][*scene_index as usize]["nodes"].as_array().expect("Scene index not found, or 'nodes' field is missing");
    if nodes_list.is_empty() {
        panic!("No nodes found in the specified scene");
    }
    println!("Found {} nodes in the scene", nodes_list.len());

    // Handle each nodes
    if nodes_list.len() > 1 {
        println!("Multiple nodes found in the scene:");
    }
    let node = &json["nodes"][nodes_list[0].as_i64().unwrap() as usize];
    println!("Processing node: {:?}", node);

    // Handle meshes in the node
    let mesh_index = node["mesh"].as_i64().expect("Node does not contain a mesh");
    let mesh = &json["meshes"][mesh_index as usize];
    println!("Processing mesh: {:?}", mesh);

    // Process primitives in the mesh (each primitive contains a description of how to draw a part of the mesh)
    let mut primitives = Vec::new();
    let raw_primitives = mesh["primitives"].as_array().expect("Mesh does not contain primitives");
    for (i, primitive) in raw_primitives.iter().enumerate() {
        println!("Processing primitive {}: {:?}", i, primitive);
        
        // Should we use indices to draw the vertices (indexed drawing)
        let vertex_indexed = primitive.get("indices").and_then(|idx| idx.as_i64());
        println!("Vertex indexed accessor index (if Some): {:?}", vertex_indexed);

        // Extract the attributes of the vertices of the mesh primitive
        let mut v_attributes = Vec::new();
        let vertex_attributes = &primitive["attributes"];
        for (attribute_name, accessor_index) in vertex_attributes.as_object().expect("Attributes should be an object") {
            let accessor_index = accessor_index.as_i64().expect("Accessor index should be an integer");
            println!("Attribute: {} uses accessor index: {}", attribute_name, accessor_index);
            v_attributes.push((attribute_name.clone(), accessor_index));
        }
        let prim = MeshPrimitive {
            vertex_indexed,
            vertex_attributes: v_attributes,
        };
        primitives.push(prim);
    }
    println!("Processed {} primitives in the mesh", primitives.len());
    println!("===> All primitives: {:?}", primitives);
    


    // GltfModel construction
    GltfModel {
        buffers: vec![
            buffer,
        ],
        meshes: vec![
            GltfMesh {
                primitives,
            }
        ],
    }
}
