//! Accessor and bufferView helpers for slicing and decoding data.

use crate::model::{AccessorData, BufferSliceData, GltfAccessorComponentType, GltfBuffer};

/// Return the size in bytes of a single component for the given `component_type`.
pub fn component_byte_size(component_type: GltfAccessorComponentType) -> usize {
    match component_type {
        GltfAccessorComponentType::Byte | GltfAccessorComponentType::UnsignedByte => 1,
        GltfAccessorComponentType::Short | GltfAccessorComponentType::UnsignedShort => 2,
        GltfAccessorComponentType::UnsignedInt | GltfAccessorComponentType::Float => 4,
    }
}

/// Return the number of components per element for the logical `accessor_type`.
pub fn accessor_components(accessor_type: &str) -> usize {
    match accessor_type {
        "SCALAR" => 1,
        "VEC2" => 2,
        "VEC3" => 3,
        "VEC4" => 4,
        _ => panic!("Unsupported accessor type: {}", accessor_type),
    }
}

/// Compute the byte range for a given accessor inside a bufferView slice.
pub fn slice_range(slice: &BufferSliceData, accessor: &AccessorData) -> (usize, usize) {
    let component_size = component_byte_size(accessor.component_type);
    let components = accessor_components(&accessor.accessor_type);
    let start = slice.byte_offset + accessor.byte_offset;
    let byte_length = accessor.count * component_size * components;
    (start, start + byte_length)
}

/// Decode indices from the given `accessor` and `buffer`.
pub fn parse_indices(accessor: &AccessorData, buffer: &GltfBuffer) -> Vec<u32> {
    let slice = &buffer.slices[accessor.buffer_view_index as usize];
    let (start, end) = slice_range(slice, accessor);
    let index_data = &buffer.data[start..end];

    match accessor.component_type {
        GltfAccessorComponentType::UnsignedShort => index_data
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]) as u32)
            .collect(),
        GltfAccessorComponentType::UnsignedInt => index_data
            .chunks_exact(4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .collect(),
        GltfAccessorComponentType::UnsignedByte => index_data.iter().map(|&b| b as u32).collect(),
        _ => panic!("Unsupported index component type"),
    }
}

/// Decode attribute data into `f32` values from the given `accessor` and `buffer`.
pub fn parse_attribute_as_f32(accessor: &AccessorData, buffer: &GltfBuffer) -> Vec<f32> {
    let slice = &buffer.slices[accessor.buffer_view_index as usize];
    let (start, end) = slice_range(slice, accessor);
    let attribute_data = &buffer.data[start..end];

    match accessor.component_type {
        GltfAccessorComponentType::Float => attribute_data
            .chunks_exact(4)
            .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .collect(),
        GltfAccessorComponentType::UnsignedShort => attribute_data
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]) as f32)
            .collect(),
        GltfAccessorComponentType::UnsignedInt => attribute_data
            .chunks_exact(4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f32)
            .collect(),
        GltfAccessorComponentType::UnsignedByte => attribute_data.iter().map(|&b| b as f32).collect(),
        _ => panic!("Unsupported component type"),
    }
}
