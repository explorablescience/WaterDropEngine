use bevy::prelude::*;
use wde_wgpu::{buffer::BufferUsage, utils::Vertex};

use crate::prelude::*;

/// Maximum number of vertices that can be stored in the SSBO buffers.
const MAX_VERTICES: usize = 1_000_000;
/// Maximum number of indices that can be stored in the SSBO buffers.
const MAX_INDICES: usize = 3_000_000;

/// Resource describing the layout of [SsboMesh] data.
#[derive(Resource)]
pub struct SsboMeshDescriptor {
    // Cursor to the current offset in the buffers (in elements, not bytes)
    pub(crate) vertex_buffer_offset: u32,
    pub(crate) index_buffer_offset: u32,

    /// Binding index of the vertex buffer in the shader
    pub ssbo_vertex_binding: u32,
    /// Binding index of the index buffer in the shader
    pub ssbo_index_binding: u32
}
impl Default for SsboMeshDescriptor {
    fn default() -> Self {
        Self {
            vertex_buffer_offset: 0,
            index_buffer_offset: 0,
            ssbo_vertex_binding: 0,
            ssbo_index_binding: 1
        }
    }
}

/// Resource representing the SSBO mesh data.
/// It is filled by every GpuMesh if use_ssbo is set to true.
/// The position of the vertex and index data in the buffers is then stored in the GpuMesh resource.
#[derive(Asset, Clone, TypePath, Default)]
pub struct SsboMesh;
impl RenderBinding for SsboMesh {
    fn describe(&self, builder: &mut RenderBindingBuilder) {
        builder.add_buffer(
            0,
            Buffer {
                label: "ssbo-mesh-vertex-buffer-gpu".to_string(),
                size: std::mem::size_of::<Vertex>() * MAX_VERTICES,
                usage: BufferUsage::STORAGE | BufferUsage::COPY_DST,
                content: None
            }
        );
        builder.add_buffer(
            1,
            Buffer {
                label: "ssbo-mesh-index-buffer-gpu".to_string(),
                size: std::mem::size_of::<u32>() * MAX_INDICES,
                usage: BufferUsage::STORAGE | BufferUsage::COPY_DST,
                content: None
            }
        );
    }
}
