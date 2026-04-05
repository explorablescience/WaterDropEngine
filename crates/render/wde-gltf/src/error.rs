//! Error types for glTF loading and parsing.

use std::fmt;

/// Errors that can occur during glTF loading and parsing.
#[derive(Debug, Clone)]
pub enum GltfError {
    /// File I/O error
    IoError(String),
    /// JSON parsing error
    JsonError(String),
    /// Unsupported glTF version
    UnsupportedVersion(String),
    /// Missing required field in JSON
    MissingField(String),
    /// Invalid buffer view index
    InvalidBufferView(i64),
    /// Buffer overflow (reading beyond buffer size)
    BufferOverflow {
        start: usize,
        end: usize,
        buffer_size: usize
    },
    /// Unsupported accessor component type
    UnsupportedComponentType(i64),
    /// Mismatched vertex counts across attributes
    MismatchedVertexCount {
        primitive: String,
        expected: usize,
        actual: usize
    },
    /// Base64 decode error
    Base64Error(String),
    /// No buffers found in glTF file
    NoBuffers,
    /// Multiple buffers (only single buffer supported)
    MultipleBuffers(usize),
    /// Invalid accessor type
    InvalidAccessorType(String),
    /// Unsupported accessor type
    UnsupportedAccessorType(String)
}

impl fmt::Display for GltfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GltfError::IoError(msg) => write!(f, "I/O error: {}", msg),
            GltfError::JsonError(msg) => write!(f, "JSON parsing error: {}", msg),
            GltfError::UnsupportedVersion(version) => {
                write!(f, "Unsupported glTF version: {}", version)
            }
            GltfError::MissingField(field) => write!(f, "Missing required field: {}", field),
            GltfError::InvalidBufferView(index) => {
                write!(f, "Invalid buffer view index: {}", index)
            }
            GltfError::BufferOverflow {
                start,
                end,
                buffer_size
            } => {
                write!(
                    f,
                    "Buffer overflow: trying to read {}..{} from buffer of size {}",
                    start, end, buffer_size
                )
            }
            GltfError::UnsupportedComponentType(type_id) => {
                write!(f, "Unsupported component type: {}", type_id)
            }
            GltfError::MismatchedVertexCount {
                primitive,
                expected,
                actual
            } => {
                write!(
                    f,
                    "Mismatched vertex count in primitive '{}': expected {}, got {}",
                    primitive, expected, actual
                )
            }
            GltfError::Base64Error(msg) => write!(f, "Base64 decode error: {}", msg),
            GltfError::NoBuffers => write!(f, "No buffers found in glTF file"),
            GltfError::MultipleBuffers(count) => write!(
                f,
                "Multiple buffers found ({}), only single buffer supported",
                count
            ),
            GltfError::InvalidAccessorType(type_name) => {
                write!(f, "Invalid accessor type: {}", type_name)
            }
            GltfError::UnsupportedAccessorType(type_name) => {
                write!(f, "Unsupported accessor type: {}", type_name)
            }
        }
    }
}

impl std::error::Error for GltfError {}

impl From<std::io::Error> for GltfError {
    fn from(err: std::io::Error) -> Self {
        GltfError::IoError(err.to_string())
    }
}

impl From<serde_json::Error> for GltfError {
    fn from(err: serde_json::Error) -> Self {
        GltfError::JsonError(err.to_string())
    }
}

impl From<base64::DecodeError> for GltfError {
    fn from(err: base64::DecodeError) -> Self {
        GltfError::Base64Error(err.to_string())
    }
}
