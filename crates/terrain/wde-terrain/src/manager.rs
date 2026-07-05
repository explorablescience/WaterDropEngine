use wde_logger::prelude::*;

use bevy::prelude::*;
use std::collections::HashMap;

use crate::utils::image_decoder::decode_png_as_channels;

/// Number of chunks along each axis (terrain is CHUNK_COUNT×CHUNK_COUNT).
pub const CHUNK_COUNT: u32 = 8;
/// Number of splat-map channels packed into one RGBA texture (always 4).
pub const SPLAT_MAP_COUNT: u32 = 4;

/// World-space size of one chunk.
pub const CHUNK_SIZE: f32 = 32.0;
/// Maximum height range in world units.
pub const CHUNK_HEIGHT: f32 = 10.0;
/// Heightmap/splatmap resolution (pixels per side).
pub const CHUNK_RENDER_SUBDIVISIONS: u32 = 256;

/// 2-D integer grid position of a terrain chunk.
pub type ChunkPos = IVec2;
/// Raw pixel bytes for one map layer.
pub(crate) type TerrainTileData = Vec<u8>;
/// (pos, map_type [0=height|1=splat], splat_index, data)
pub(crate) type TerrainDirtyTile = (ChunkPos, u32, u32, TerrainTileData);

/// Source-of-truth terrain component.
/// Owns the dirty queues consumed by the render and physics sub-systems.
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Terrain {
    pub path: String,
    /// Maps chunk grid position → sequential index (used by UI for iteration).
    pub pos_to_tile: HashMap<ChunkPos, usize>,

    /// Tiles that need re-uploading to the GPU this frame.
    #[reflect(ignore)]
    pub(crate) dirty_render: Vec<TerrainDirtyTile>,
    /// Heightmap tiles that need their physics collider rebuilt.
    #[reflect(ignore)]
    pub dirty_physics: Vec<TerrainDirtyTile>
}

impl Terrain {
    /// Load terrain data from `res/<path>/`.
    pub fn load(path: &str) -> Self {
        let cur_dir = std::env::current_dir().unwrap();
        let full_path = format!("{}/res/{}", cur_dir.display(), path);

        let mut pos_to_tile = HashMap::new();
        let mut dirty_render = Vec::new();
        let mut dirty_physics = Vec::new();
        let mut index = 0usize;

        for i in 0..CHUNK_COUNT {
            for j in 0..CHUNK_COUNT {
                let px = i as i32 - (CHUNK_COUNT as i32) / 2;
                let pz = j as i32 - (CHUNK_COUNT as i32) / 2;
                let pos = IVec2::new(px, pz);

                // --- heightmap ---
                let hm_path =
                    if std::fs::metadata(format!("{}/heightmap_{}_{}.png", full_path, px, pz))
                        .is_ok()
                    {
                        format!("{}/heightmap_{}_{}.png", full_path, px, pz)
                    } else {
                        format!("{}/heightmap_default.png", full_path)
                    };

                let heightmap = match decode_png_as_channels(
                    &hm_path,
                    1,
                    (CHUNK_RENDER_SUBDIVISIONS, CHUNK_RENDER_SUBDIVISIONS)
                ) {
                    Ok(d) => d,
                    Err(e) => {
                        error!("Failed to load heightmap ({}, {}): {}", px, pz, e);
                        continue;
                    }
                };

                dirty_render.push((pos, 0, 0, heightmap.clone()));
                dirty_physics.push((pos, 0, 0, heightmap));

                // --- splatmap (one RGBA texture = 4 layers) ---
                let sm_path =
                    if std::fs::metadata(format!("{}/splatmap_{}_{}-0.png", full_path, px, pz))
                        .is_ok()
                    {
                        format!("{}/splatmap_{}_{}-0.png", full_path, px, pz)
                    } else {
                        format!("{}/splatmap_default.png", full_path)
                    };

                let splatmap = match decode_png_as_channels(
                    &sm_path,
                    4,
                    (CHUNK_RENDER_SUBDIVISIONS, CHUNK_RENDER_SUBDIVISIONS)
                ) {
                    Ok(d) => d,
                    Err(e) => {
                        error!("Failed to load splatmap ({}, {}): {}", px, pz, e);
                        continue;
                    }
                };

                dirty_render.push((pos, 1, 0, splatmap));
                pos_to_tile.insert(pos, index);
                index += 1;
            }
        }

        Self {
            path: path.to_string(),
            pos_to_tile,
            dirty_render,
            dirty_physics
        }
    }

    /// Returns the chunk grid position for a world position, or `None` if out of bounds.
    pub fn get_tile_idx_for_world_pos(&self, world_pos: Vec3) -> Option<IVec2> {
        let tile_pos = IVec2::new(
            (world_pos.x / CHUNK_SIZE).round() as i32,
            (world_pos.z / CHUNK_SIZE).round() as i32
        );
        self.pos_to_tile.contains_key(&tile_pos).then_some(tile_pos)
    }
}
