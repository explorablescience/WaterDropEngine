use std::collections::HashMap;

use bevy::prelude::*;
use wde_renderer::prelude::*;

use crate::manager::{CHUNK_COUNT, CHUNK_RENDER_SUBDIVISIONS, ChunkPos, Terrain, TerrainDirtyTile};

/// CPU-side terrain renderer.
/// Holds the two shared texture arrays (one heightmap array, one splatmap array) and
/// the dirty queue that gets forwarded to the GPU each frame.
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct TerrainRenderer {
    /// Maps chunk position → array layer index (= tile index).
    pub pos_to_layer: HashMap<ChunkPos, u32>,
    /// Single R8Unorm texture array: one layer per chunk.
    #[reflect(ignore)]
    pub heightmap_array: Option<Handle<Texture>>,
    /// Single Rgba8Unorm texture array: one layer per chunk.
    #[reflect(ignore)]
    pub splatmap_array: Option<Handle<Texture>>,
    /// Dirty tiles forwarded from `Terrain` each frame.
    #[reflect(ignore)]
    pub dirty: Vec<TerrainDirtyTile>
}

impl TerrainRenderer {
    /// Create the two shared texture arrays.
    pub fn new(asset_server: &AssetServer) -> Self {
        let layer_count = CHUNK_COUNT * CHUNK_COUNT;
        let usages = TextureUsages::COPY_SRC
            | TextureUsages::COPY_DST
            | TextureUsages::TEXTURE_BINDING
            | TextureUsages::STORAGE_BINDING;

        let heightmap_array = asset_server.add(Texture {
            label: "terrain_heightmap_array".to_string(),
            size: (CHUNK_RENDER_SUBDIVISIONS, CHUNK_RENDER_SUBDIVISIONS),
            format: TextureFormat::R8Unorm,
            usages,
            layer_count,
            ..Default::default()
        });

        let splatmap_array = asset_server.add(Texture {
            label: "terrain_splatmap_array".to_string(),
            size: (CHUNK_RENDER_SUBDIVISIONS, CHUNK_RENDER_SUBDIVISIONS),
            format: TextureFormat::Rgba8Unorm,
            usages,
            layer_count,
            ..Default::default()
        });

        // Build the position→layer map in the same iteration order used by Terrain::load.
        let mut pos_to_layer = HashMap::new();
        let mut layer = 0u32;
        for i in 0..CHUNK_COUNT {
            for j in 0..CHUNK_COUNT {
                let px = i as i32 - (CHUNK_COUNT as i32) / 2;
                let pz = j as i32 - (CHUNK_COUNT as i32) / 2;
                pos_to_layer.insert(IVec2::new(px, pz), layer);
                layer += 1;
            }
        }

        Self {
            pos_to_layer,
            heightmap_array: Some(heightmap_array),
            splatmap_array: Some(splatmap_array),
            dirty: Vec::new()
        }
    }

    /// Move dirty tiles from `Terrain` into this renderer's queue.
    pub fn extract_dirty(
        mut renderer: Query<&mut TerrainRenderer>,
        mut terrain: Query<&mut Terrain>
    ) {
        let (Ok(mut renderer), Ok(mut terrain)) = (renderer.single_mut(), terrain.single_mut())
        else {
            return;
        };
        renderer.dirty.clear();
        renderer.dirty.append(&mut terrain.dirty_render);
    }
}
