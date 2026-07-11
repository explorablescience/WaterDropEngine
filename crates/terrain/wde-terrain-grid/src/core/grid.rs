use bevy::prelude::*;
use std::collections::HashMap;
use wde_terrain::prelude::{CHUNK_SIZE, ChunkPos};

use crate::prelude::GridEntity;

/// The number of subdivisions per chunk in the grid system (number of grid cells per chunk).
pub const CHUNK_GRID_SUBDIVISIONS: u32 = CHUNK_SIZE as u32 / 2;
/// The size of a single tile in the grid system.
pub const TILE_SIZE: f32 = CHUNK_SIZE / CHUNK_GRID_SUBDIVISIONS as f32;

/// The position of a chunk in the grid, represented as (x, z) coordinates.
pub type GridChunkPos = IVec2;
/// The local position of a tile within a chunk, represented as (x, z) coordinates.
type GridLocalPos = (u32, u32);
/// The full position in the grid
pub type GridTilePos = (GridChunkPos, GridLocalPos);

/// A terrain chunk that contains a list of terrain tiles.
pub struct Chunk {
    /// List of terrain tiles that belong to this chunk.
    pub tiles: Vec<Option<Entity>>,
    /// Pointers from the entities to their position in the grid.
    pub entity_to_tile: HashMap<Entity, Vec<GridLocalPos>>
}
impl Default for Chunk {
    fn default() -> Self {
        Chunk {
            tiles: vec![None; (CHUNK_GRID_SUBDIVISIONS * CHUNK_GRID_SUBDIVISIONS * 4) as usize],
            entity_to_tile: HashMap::new()
        }
    }
}
impl Chunk {
    // Entity management methods
    /// Sets the entity at the specified local tile position within this chunk.
    pub fn set_entity_at(&mut self, local_pos: GridLocalPos, entity: Entity) {
        let index = Self::local_pos_to_index(local_pos);
        if let Some(tile) = self.tiles.get_mut(index) {
            *tile = Some(entity);
            self.entity_to_tile
                .entry(entity)
                .or_default()
                .push(local_pos);
        }
    }
    /// Gets the entity at the specified local tile position within this chunk, if it exists.
    pub fn get_entity_at(&self, local_pos: GridLocalPos) -> Option<Entity> {
        let index = Self::local_pos_to_index(local_pos);
        self.tiles.get(index).and_then(|tile| *tile)
    }
    /// Removes the entity from all tiles it occupies in this chunk.
    pub fn remove_entity(&mut self, entity: Entity) {
        if let Some(local_positions) = self.entity_to_tile.remove(&entity) {
            for local_pos in local_positions {
                let index = Self::local_pos_to_index(local_pos);
                if let Some(tile) = self.tiles.get_mut(index)
                    && tile.as_ref() == Some(&entity)
                {
                    *tile = None;
                }
            }
        }
    }

    // Coordinate conversion methods
    /// Converts a world position to a local tile position within this chunk.
    pub fn get_nearest_local_tile(world_pos: Vec2) -> GridLocalPos {
        let half_chunk = CHUNK_SIZE * 0.5;
        let cell_size = CHUNK_SIZE / CHUNK_GRID_SUBDIVISIONS as f32;

        let local_x = (world_pos.x + half_chunk).rem_euclid(CHUNK_SIZE) / cell_size;
        let local_y = (world_pos.y + half_chunk).rem_euclid(CHUNK_SIZE) / cell_size;

        (local_x.floor() as u32, local_y.floor() as u32)
    }
    /// Gets the center position of a tile.
    pub fn get_tile_world_pos(pos: GridTilePos) -> Vec2 {
        let (chunk_pos, local_pos) = pos;
        chunk_pos.as_vec2() * CHUNK_SIZE
            + Vec2::new(local_pos.0 as f32, local_pos.1 as f32) * TILE_SIZE
            + TILE_SIZE / 2.0
            - CHUNK_SIZE / 2.0
    }
    /// Converts a local tile position to an index in the tiles vector.
    fn local_pos_to_index(local_pos: GridLocalPos) -> usize {
        let (local_x, local_y) = local_pos;
        (local_y * CHUNK_GRID_SUBDIVISIONS + local_x) as usize * 4
    }
}

/// The main terrain grid resource that holds all the terrain chunks and their respective tiles.
#[derive(Resource, Default)]
pub struct Grid {
    chunks: HashMap<GridChunkPos, Chunk>,
    parent: Option<Entity>
}
impl Grid {
    /// Gets the entity at the specified chunk and local tile position in the grid, if it exists.
    pub fn get_entity(&self, chunk_pos: GridChunkPos, local_pos: GridLocalPos) -> Option<Entity> {
        self.chunks
            .get(&chunk_pos)
            .and_then(|chunk| chunk.get_entity_at(local_pos))
    }
    /// Adds the specified entity to the grid at the positions occupied by the entity footprint.
    pub fn set_entity(&mut self, grid_entity: &GridEntity, entity: Entity) {
        for pos in grid_entity.footprint() {
            let chunk = self.chunks.entry(pos.0).or_default();
            chunk.set_entity_at(pos.1, entity);
        }
    }
    /// Removes the specified entity from the grid, freeing up the tiles it occupied.
    pub fn remove_entity(&mut self, entity: Entity) {
        for chunk in self.chunks.values_mut() {
            chunk.remove_entity(entity);
        }
    }
    /// Check if the position and extent are valid for placement (no existing entities in the area).
    pub fn is_area_free(&self, entity: &GridEntity) -> bool {
        let footprint = entity.footprint();
        for pos in footprint {
            if self.get_entity(pos.0, pos.1).is_some() {
                return false;
            }
        }
        true
    }

    // Methods to convert between world positions and chunk/local positions
    /// Gets the nearest chunk and tile position (without subtile direction) for a given world position.
    pub fn get_nearest_tile(world_pos: Vec2) -> GridTilePos {
        let half_chunk = CHUNK_SIZE * 0.5;
        let chunk_pos = ChunkPos::new(
            (world_pos.x + half_chunk).div_euclid(CHUNK_SIZE) as i32,
            (world_pos.y + half_chunk).div_euclid(CHUNK_SIZE) as i32
        );
        let local_pos = Chunk::get_nearest_local_tile(world_pos);
        (chunk_pos, local_pos)
    }
    /// Gets the center world position of a given chunk.
    pub fn get_chunk_world_pos(chunk_pos: GridChunkPos) -> Vec2 {
        Vec2::new(
            chunk_pos.x as f32 * CHUNK_SIZE * 2.0,
            chunk_pos.y as f32 * CHUNK_SIZE * 2.0
        )
    }
    /// Gets the center world position for a given chunk and local tile position.
    pub fn get_tile_world_pos(pos: GridTilePos) -> Vec2 {
        Chunk::get_tile_world_pos(pos)
    }

    // Helper methods
    pub fn set_parent(&mut self, parent: Entity) {
        self.parent = Some(parent);
    }
    pub fn get_parent(&self) -> Option<Entity> {
        self.parent
    }
}
