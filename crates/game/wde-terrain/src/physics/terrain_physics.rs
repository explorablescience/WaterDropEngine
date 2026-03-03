use std::collections::HashMap;

use wde_physics::prelude::*;
use bevy::prelude::*;

use crate::manager::{PHYSICS_TILE_SUBDIVISIONS, TILE_SIZE, Terrain, TilePos};

#[derive(Component, Default)]
pub struct TerrainPhysics {
    /// Pointer from tile position (x, z) to the corresponding tile entity
    pub pos_to_entity: HashMap<TilePos, Entity>
}
impl TerrainPhysics {
    /// Extracts the dirty tiles from the main terrain
    pub fn extract_dirty(mut commands: Commands, mut renderer: Query<&mut TerrainPhysics>, terrain: Query<&Terrain>) {
        let mut terrain_renderer = match renderer.iter_mut().next() {
            Some(terrain) => terrain,
            None => return,
        };
        let terrain = match terrain.iter().next() {
            Some(terrain) => terrain,
            None => return,
        };

        // Extract the dirty tiles from the main terrain and create the according colliders
        for (tile_pos, tile_type, _, data) in &terrain.dirty {
            // Check if the tile is a heightmap
            if *tile_type != 0 {
                continue;
            }

            // Format the height data
            let ss = PHYSICS_TILE_SUBDIVISIONS;
            let mut heights = vec![0.0; ss as usize * ss as usize];
            for i in 0..ss {
                for j in 0..ss {
                    let idx = (i * ss + j) as usize;
                    heights[idx] = data[idx] as f32 / 255.0;
                }
            }

            // Create the heightfield collider for the tile
            let collider = Collider::heightfield(heights, TILE_SIZE);

            // Spawn or update the collider entity for the tile
            if let Some(entity) = terrain_renderer.pos_to_entity.get(tile_pos) {
                commands.entity(*entity).insert(collider);
            } else {
                let entity = commands.spawn((
                    Transform::from_xyz(tile_pos.x as f32 * TILE_SIZE[0], 0.0, tile_pos.y as f32 * TILE_SIZE[2]),
                    collider
                )).id();
                terrain_renderer.pos_to_entity.insert(*tile_pos, entity);
            }
        }
    }
}
