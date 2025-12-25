use bevy::{platform::collections::HashMap, prelude::*};
use wde_render::assets::Buffer;
use wde_wgpu::bind_group::WgpuBindGroup;


// =========== CHUNK INDEX ===========
pub type MCChunkIndex = (i32, i32, i32);


// =========== MARCHING CUBES CHUNK DESCRIPTION ===========
/**
 * Description of a chunk for the marching cubes algorithm.
 * This description is required for each state of a chunk.
 */
#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct MCChunkDescription {
    /// Index of the chunk
    pub index: MCChunkIndex,
    /// Translation of the center of the chunk
    pub translation: Vec3,
    /// Length of the chunk in each axis
    pub length: Vec3,
    /// Number of sub-chunks in each axis
    pub sub_count: UVec3,
    /// Iso level for the marching cubes algorithm
    pub iso_level: f32
}


// =========== MARCHING CUBES INTERFACE ===========
/** List of all chunks. */
#[derive(Resource, Default)]
pub struct MCChunksListMain {
    /** List of all currently alive chunks. */
    pub current_chunks: HashMap<MCChunkIndex, MCChunkIndex>,
    /** List of new chunks to spawn. */
    pub new_chunks: Vec<(MCChunkIndex, MCChunkDescription)>,
    /** List of old chunks to delete. */
    pub delete_chunks: Vec<MCChunkIndex>
}
/** List of all chunks. */
#[derive(Resource, Default)]
pub struct MCChunksListRender {
    pub chunks: HashMap<MCChunkIndex, MCChunkDescription>
}



// =========== MARCHING CUBES CHUNK STATES IN RENDER THREAD ============
// ============ Registered => Loading => Pending => Active =============

/** Chunk waiting to generate its points. */
#[derive(Component, Default)]
pub struct MCRegisteredChunk {
    pub index: MCChunkIndex,

    // List of points
    pub points_gpu: Handle<Buffer>,
    pub points_gpu_group: Option<WgpuBindGroup>
}

/** Chunk waiting for the compute shader to generate the triangles. */
#[derive(Component)]
pub struct MCLoadingChunk {
    pub index: MCChunkIndex,

    // List of points
    pub points_gpu: Handle<Buffer>,
    pub points_gpu_group: Option<WgpuBindGroup>,
}
impl Clone for MCLoadingChunk {
    fn clone(&self) -> Self {
        if self.points_gpu_group.is_some() {
            error!("Cannot clone a chunk with a bind group");
        }
        Self {
            index: self.index,
            points_gpu: self.points_gpu.clone(),
            points_gpu_group: None
        }
    }
}

/** Chunk waiting for the mesh and physics collision generation. */
#[derive(Component)]
pub struct MCPendingChunk {
    pub index: MCChunkIndex,

    // Raw triangles after compute shader
    pub raw_triangles: Vec<f32>,
    pub triangles_counter: u32,

    // List of points
    pub points_gpu: Handle<Buffer>
}

/** Chunk ready to be rendered. */
#[derive(Component)]
#[allow(dead_code)]
pub struct MCActiveChunk {
    pub index: MCChunkIndex,

    // Vertices and indices buffers
    pub vertices: Handle<Buffer>,
    pub indices: Handle<Buffer>,
    pub indices_counter: u32,
    
    // List of points
    pub points_gpu: Handle<Buffer>
}
