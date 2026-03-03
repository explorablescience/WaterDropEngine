use std::collections::HashMap;

use bevy::prelude::*;

pub struct TerrainTileCollider {
    pub vertices: Vec<[f32; 3]>,
    pub indices: Vec<u32>
}

#[derive(Component)]
pub struct TerrainCollider {
    // Pointer from position (x, z) to the corresponding tile datas
    pub tile_map: HashMap<IVec2, usize>,
    // A list of terrain tiles that make up the entire terrain
    pub tiles: Vec<TerrainTileCollider>,

    // The list of indices that are dirty and need to be re-uploaded to the GPU
    pub dirty_tiles: Vec<IVec2>
}
impl TerrainCollider {
    pub fn new(path: &str, terrain_size: u8) -> Self {
        Self { tile_map: HashMap::new(), tiles: Vec::new(), dirty_tiles: Vec::new() }
    }
}
