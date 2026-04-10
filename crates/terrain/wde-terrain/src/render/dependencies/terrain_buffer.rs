use bevy::prelude::*;
use wde_renderer::prelude::*;

use crate::{
    manager::{CHUNK_HEIGHT, CHUNK_RENDER_SUBDIVISIONS, CHUNK_SIZE},
    render::renderer_gpu::TerrainRendererGPU
};

// The maximum number of terrain tiles that can be rendered
const MAX_TERRAIN_TILES: usize = 1000;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TerrainDescription {
    pub tile_size: [f32; 3],
    pub tile_subdivisions: f32
}
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TerrainTileDescription {
    pub pos: [f32; 2],
    pub lod: f32,
    pub _padding: f32
}

/// Struct to hold the terrain uniform layout description.
#[derive(Asset, Clone, Debug, Default, TypePath)]
pub struct TerrainBuffer;
impl TerrainBuffer {
    pub const DESC_BIND: u32 = 0;
    pub const TILES_BIND: u32 = 1;
}
impl RenderBindingOld for TerrainBuffer {
    fn describe(&self, builder: &mut RenderBindingBuilderOld) {
        builder.add_buffer(
            Self::DESC_BIND,
            Buffer {
                label: "ssbo-terrain-description-buffer".to_string(),
                size: std::mem::size_of::<TerrainDescription>(),
                usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST,
                content: Some(
                    bytemuck::cast_slice(&[TerrainDescription {
                        tile_size: [CHUNK_SIZE, CHUNK_HEIGHT, CHUNK_SIZE],
                        tile_subdivisions: CHUNK_RENDER_SUBDIVISIONS as f32
                    }])
                    .into()
                )
            }
        );
        builder.add_buffer(
            Self::TILES_BIND,
            Buffer {
                label: "ssbo-terrain-tiles-buffer".to_string(),
                size: std::mem::size_of::<TerrainTileDescription>() * MAX_TERRAIN_TILES,
                usage: BufferUsage::STORAGE | BufferUsage::COPY_DST,
                content: None
            }
        );
    }

    fn label(&self) -> &str {
        "terrain"
    }
}

// System to update the terrain tiles buffer with the current visible tiles
pub(crate) fn update_terrain_tiles_buffer(
    render_instance: Res<RenderInstance>,
    terrain_buffer: BindingOld<TerrainBuffer>,
    buffers: Res<RenderAssets<GpuBuffer>>,
    terrain_tiles: Res<TerrainRendererGPU>,
    mut is_set: Local<bool>
) {
    // Check if is ready
    if *is_set || !terrain_tiles.ready {
        return;
    }

    // Get the buffer
    let terrain_buffer = match terrain_buffer.iter().next() {
        Some((_, buffer)) => buffer,
        None => return
    };
    let tile_buffer = match buffers.get(
        terrain_buffer
            .get_buffer(TerrainBuffer::TILES_BIND)
            .unwrap()
    ) {
        Some(buffer) => buffer,
        None => return
    };

    // Prepare the data
    let data: Vec<TerrainTileDescription> = terrain_tiles
        .tiles
        .iter()
        .map(|tile| TerrainTileDescription {
            pos: [tile.position.x as f32, tile.position.y as f32],
            lod: 1.0,
            _padding: 0.0
        })
        .collect();

    // Update the buffer
    let render_instance = render_instance.0.read().unwrap();
    tile_buffer
        .buffer
        .write(&render_instance, bytemuck::cast_slice(&data), 0);

    *is_set = true;
}
