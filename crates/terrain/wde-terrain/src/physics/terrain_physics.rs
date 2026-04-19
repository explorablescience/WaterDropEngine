use std::collections::HashMap;

use bevy::prelude::*;
use wde_physics::prelude::*;

use crate::manager::{CHUNK_HEIGHT, CHUNK_SIZE, ChunkPos, Terrain};

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub struct TerrainPhysics {
    /// Super parent to all tile colliders, used for better organization in the scene hierarchy
    pub parent: Option<Entity>,
    /// Pointer from tile position (x, z) to the corresponding tile entity
    pub pos_to_entity: HashMap<ChunkPos, Entity>
}
impl TerrainPhysics {
    /// Extracts the dirty tiles from the main terrain
    pub fn extract_dirty(
        mut commands: Commands,
        mut renderer: Query<&mut TerrainPhysics>,
        mut terrain: Query<(Entity, &mut Terrain)>
    ) {
        let mut terrain_renderer = match renderer.iter_mut().next() {
            Some(terrain) => terrain,
            None => return
        };
        let (terrain_entity, mut terrain) = match terrain.iter_mut().next() {
            Some(terrain) => terrain,
            None => return
        };

        // Ensure the parent entity for the colliders exists
        if terrain_renderer.parent.is_none() {
            let parent_entity = commands
                .spawn((Name::new("Terrain Colliders"), ChildOf(terrain_entity)))
                .id();
            terrain_renderer.parent = Some(parent_entity);
        }

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
                let entity = commands
                    .spawn((
                        Name::new(format!(
                            "Terrain TileCollider ({}, {})",
                            tile_pos.x, tile_pos.y
                        )),
                        Transform::from_xyz(
                            tile_pos.x as f32 * CHUNK_SIZE,
                            0.0,
                            tile_pos.y as f32 * CHUNK_SIZE
                        ),
                        collider
                    ))
                    .id();
                commands
                    .entity(entity)
                    .set_parent_in_place(terrain_renderer.parent.unwrap());
                terrain_renderer.pos_to_entity.insert(*tile_pos, entity);
            }
        }

        // Clear the dirty tiles list after processing
        terrain.dirty_physics.clear();
    }
}
