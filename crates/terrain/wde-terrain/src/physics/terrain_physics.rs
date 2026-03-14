use std::collections::HashMap;

use wde_physics::prelude::*;
use bevy::prelude::*;

use crate::manager::{CHUNK_HEIGHT, CHUNK_SIZE, ChunkPos, Terrain};

#[derive(Component, Default)]
pub struct TerrainPhysics {
    /// Pointer from tile position (x, z) to the corresponding tile entity
    pub pos_to_entity: HashMap<ChunkPos, Entity>
}
impl TerrainPhysics {
    /// Extracts the dirty tiles from the main terrain
    pub fn extract_dirty(mut commands: Commands, mut renderer: Query<&mut TerrainPhysics>, mut terrain: Query<&mut Terrain>) {
        let mut terrain_renderer = match renderer.iter_mut().next() {
            Some(terrain) => terrain,
            None => return,
        };
        let mut terrain = match terrain.iter_mut().next() {
            Some(terrain) => terrain,
            None => return,
        };

        // Extract the dirty tiles from the main terrain and create the according colliders
        for (tile_pos, tile_type, _, data) in terrain.dirty_physics.iter().flatten() {
            // Check if the tile is a heightmap
            if *tile_type != 0 {
                continue;
            }

            // Convert the tile data from bytes to f32 heights
            let mut heights = vec![0.0; data.len()];
            for idx in 0..data.len() {
                heights[idx] = data[idx] as f32 / 255.0;
            }

            // Create the heightfield collider for the tile
            let collider = Collider::heightfield(heights, [CHUNK_SIZE, CHUNK_HEIGHT, CHUNK_SIZE]);

            // Spawn or update the collider entity for the tile
            if let Some(entity) = terrain_renderer.pos_to_entity.get(tile_pos) {
                commands.entity(*entity).insert(collider);
            } else {
                let entity = commands.spawn((
                    Transform::from_xyz(tile_pos.x as f32 * CHUNK_SIZE, 0.0, tile_pos.y as f32 * CHUNK_SIZE),
                    collider
                )).id();
                terrain_renderer.pos_to_entity.insert(*tile_pos, entity);
            }
        }

        // Clear the dirty tiles list after processing
        terrain.dirty_physics.clear();
    }
}
