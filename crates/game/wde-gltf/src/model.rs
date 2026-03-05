//! Core glTF model structures used by the loader and parser.
#![allow(dead_code)]

use bevy::prelude::*;

use crate::material::GltfMaterial;

/// Definitions of glTF model, buffers, meshes, accessors, and materials.
#[derive(Debug, Clone)]
pub struct GltfModel {
    pub path: String,
    pub filename: String,
    pub buffers: Vec<GltfBuffer>,
    pub meshes: Vec<GltfMesh>,
    pub materials: Vec<GltfMaterial>
}

/// Representation of a glTF buffer with its raw data and bufferView slices.
#[derive(Clone)]
pub struct GltfBuffer {
    pub data: Vec<u8>,
    pub slices: Vec<BufferSliceData>,
}
impl std::fmt::Debug for GltfBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GltfBuffer")
            .field("data_length", &self.data.len())
            .field("slices", &self.slices)
            .finish()
    }
}

/// Representation of a bufferView slice within a glTF buffer.
#[derive(Debug, Clone)]
pub struct BufferSliceData {
    /// Byte offset within the buffer for this view
    pub byte_offset: usize
}

/// Representation of a glTF mesh with its primitives.
#[derive(Debug, Clone)]
pub struct GltfMesh {
    pub primitives: Vec<MeshPrimitive>
}

/// Representation of a glTF mesh primitive with its attributes and indices.
#[derive(Debug, Clone)]
pub struct MeshPrimitive {
    /// Name of the primitive (if any)
    pub name: String,
    /// Optional indices accessor if the primitive is indexed
    pub vertex_indexed: Option<AccessorData>,
    /// Map of attribute name to its accessor data
    pub vertex_attributes: Vec<(String, AccessorData)>,
    /// Material index of the primitive
    pub material_id: Option<u32>,
    /// Node translation [x, y, z]
    pub translation: [f32; 3],
    /// Node rotation as quaternion [x, y, z, w]
    pub rotation: [f32; 4],
    /// Node scale [x, y, z]
    pub scale: [f32; 3],
}

/// Representation of a glTF accessor for accessing buffer data.
#[derive(Debug, Clone)]
pub struct AccessorData {
    /// Index of the bufferView this accessor references
    pub buffer_view_index: i64,
    /// Additional byte offset within the bufferView
    pub byte_offset: usize,
    /// Component type of the accessor values
    pub component_type: GltfAccessorComponentType,
    /// Number of elements in the accessor
    pub count: usize,
    /// Logical accessor type (e.g., SCALAR, VEC2, VEC3, ...)
    pub accessor_type: String,
    /// Optional minimum values (for bounding box computation)
    pub min: Option<Vec<f32>>,
    /// Optional maximum values (for bounding box computation)
    pub max: Option<Vec<f32>>,
}

/// Enumeration of glTF accessor component types.
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
    /// Create a GltfAccessorComponentType from its integer value.
    pub fn from_i64(value: i64) -> Option<Self> {
        match value {
            5120 => Some(GltfAccessorComponentType::Byte),
            5121 => Some(GltfAccessorComponentType::UnsignedByte),
            5122 => Some(GltfAccessorComponentType::Short),
            5123 => Some(GltfAccessorComponentType::UnsignedShort),
            5125 => Some(GltfAccessorComponentType::UnsignedInt),
            5126 => Some(GltfAccessorComponentType::Float),
            _ => unreachable!("Unsupported component type: {}", value),
        }
    }
}
