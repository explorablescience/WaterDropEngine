use std::collections::HashMap;

use bevy::prelude::*;
use wde_physics::prelude::*;

use crate::{
    TERRAIN_COLLIDER_GROUP,
    manager::{CHUNK_HEIGHT, CHUNK_SIZE, ChunkPos, Terrain}
};

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub struct TerrainPhysics {
    pub parent: Option<Entity>,
    pub pos_to_entity: HashMap<ChunkPos, Entity>
}

impl TerrainPhysics {
    pub fn extract_dirty(
        mut commands: Commands,
        mut physics: Query<&mut TerrainPhysics>,
        mut terrain: Query<(Entity, &mut Terrain)>
    ) {
        let (Ok(mut physics), Ok((terrain_entity, mut terrain))) =
            (physics.single_mut(), terrain.single_mut())
        else {
            return;
        };

        // Lazily create the parent collider container.
        if physics.parent.is_none() {
            physics.parent = Some(
                commands
                    .spawn((
                        Name::new("Terrain Colliders"),
                        Transform::default(),
                        ChildOf(terrain_entity)
                    ))
                    .id()
            );
        }
        let parent = physics.parent.unwrap();

        for (tile_pos, tile_type, _, data) in terrain.dirty_physics.drain(..) {
            if tile_type != 0 {
                continue; // Only heightmaps drive colliders.
            }

            let collider = Collider::from(
                HeightfieldCollider::from_heightmap(&data, [CHUNK_SIZE, CHUNK_HEIGHT, CHUNK_SIZE])
                    .with_group(TERRAIN_COLLIDER_GROUP)
            );

            if let Some(&entity) = physics.pos_to_entity.get(&tile_pos) {
                commands.entity(entity).insert(collider);
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
                        collider,
                        ChildOf(parent)
                    ))
                    .id();
                physics.pos_to_entity.insert(tile_pos, entity);
            }
        }
    }
}
