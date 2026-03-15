use std::collections::HashMap;
use bevy::prelude::*;
use wde_terrain::prelude::{CHUNK_SIZE, ChunkPos};

/// The number of subdivisions per chunk in the grid system (number of grid cells per chunk).
pub const CHUNK_GRID_SUBDIVISIONS: u32 = CHUNK_SIZE as u32 / 2;

/// The position of a chunk in the grid, represented as (x, z) coordinates.
pub type GridChunkPos = IVec2;
/// The local position of a tile within a chunk (x, z coordinates and a direction).
pub type GridLocalPos = (u32, u32, GridLocalDir);
/// The full position in the grid
pub type GridPos = (GridChunkPos, GridLocalPos);
/// The local position of a tile within a chunk (x, z coordinates), without subtile direction.
pub type GridTileLocalPos = (u32, u32);
/// The tile position in the grid, without subtile direction.
pub type GridTilePos = (GridChunkPos, GridTileLocalPos);
/// The local position of a tile within a square tile.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GridLocalDir {
    North,
    East,
    West,
    South
}


/// A terrain chunk that contains a list of terrain tiles.
#[derive(Default)]
pub struct Chunk {
    /// The position of the chunk in the grid.
    pub pos: GridChunkPos,
    /// List of terrain tiles that belong to this chunk.
    /// The tile is None if it contains no data. If it is Some, the Entity should have a TerrainTileData component with the corresponding data.
    pub tiles: Vec<Option<Entity>>
}
impl Chunk {
    /// Creates a new terrain chunk with the specified position and an empty tile list.
    pub fn new(pos: GridChunkPos) -> Self {
        let tiles = vec![None; (CHUNK_GRID_SUBDIVISIONS * CHUNK_GRID_SUBDIVISIONS * 4) as usize];
        Chunk { pos, tiles }
    }


    // Entity management methods
    /// Sets the entity at the specified local tile position within this chunk.
    pub fn set_entity_at(&mut self, local_pos: GridLocalPos, entity: Entity) {
        let index = Self::local_pos_to_index(local_pos);
        if let Some(tile) = self.tiles.get_mut(index) {
            *tile = Some(entity);
        }
    }
    /// Gets the entity at the specified local tile position within this chunk, if it exists.
    pub fn get_entity_at(&self, local_pos: GridLocalPos) -> Option<Entity> {
        let index = Self::local_pos_to_index(local_pos);
        self.tiles.get(index).and_then(|tile| *tile)
    }
    /// Clears the entity at the specified local tile position within this chunk.
    pub fn clear_entity_at(&mut self, local_pos: GridLocalPos) {
        let index = Self::local_pos_to_index(local_pos);
        if let Some(tile) = self.tiles.get_mut(index) {
            *tile = None;
        }
    }

    // Coordinate conversion methods
    /// Converts a world position to a local tile position within this chunk.
    pub fn world_to_local(world_pos: Vec2) -> GridLocalPos {
        let half_chunk = CHUNK_SIZE * 0.5;
        let cell_size = CHUNK_SIZE / CHUNK_GRID_SUBDIVISIONS as f32;

        let local_x = (world_pos.x + half_chunk).rem_euclid(CHUNK_SIZE) / cell_size;
        let local_y = (world_pos.y + half_chunk).rem_euclid(CHUNK_SIZE) / cell_size;

        let frac_x = local_x.fract() - 0.5;
        let frac_y = local_y.fract() - 0.5;

        let local_dir = if frac_y.abs() >= frac_x.abs() {
            if frac_y >= 0.0 {
                GridLocalDir::North
            } else {
                GridLocalDir::South
            }
        } else if frac_x >= 0.0 {
            GridLocalDir::East
        } else {
            GridLocalDir::West
        };

        (local_x.floor() as u32, local_y.floor() as u32, local_dir)
    }
    /// Converts a local tile position to a world position. This will select the center of the subtile.
    pub fn local_to_world(chunk_pos: GridChunkPos, local_pos: GridLocalPos) -> Vec2 {
        let cell_s = CHUNK_SIZE / CHUNK_GRID_SUBDIVISIONS as f32;
        let cell_pos = chunk_pos.as_vec2() * CHUNK_SIZE + Vec2::new(local_pos.0 as f32, local_pos.1 as f32) * cell_s + cell_s / 2.0 - CHUNK_SIZE / 2.0;
        let rot = match local_pos.2 {
            GridLocalDir::North => 0.0,
            GridLocalDir::South => std::f32::consts::PI,
            GridLocalDir::West => -std::f32::consts::PI / 2.0,
            GridLocalDir::East => std::f32::consts::PI / 2.0
        };
        let tr = match local_pos.2 {
            GridLocalDir::North => Vec2::new(0.0, 1.0),
            GridLocalDir::South => Vec2::new(0.0, -1.0),
            GridLocalDir::West => Vec2::new(-1.0, 0.0),
            GridLocalDir::East => Vec2::new(1.0, 0.0)
        } / 2.0;
        Vec2::new(
            crate::ops::cos(rot),
            crate::ops::sin(rot)
        ) * cell_s + cell_pos + tr
    }

    // Helper methods
    /// Gets a reference to the tiles vector in this chunk.
    pub fn get_tiles(&self) -> &[Option<Entity>] {
        &self.tiles
    }
    /// Converts a local tile position to an index in the tiles vector.
    fn local_pos_to_index(local_pos: GridLocalPos) -> usize {
        let (local_x, local_y, local_dir) = local_pos;
        let dir_offset = match local_dir {
            GridLocalDir::North => 0,
            GridLocalDir::East => 1,
            GridLocalDir::West => 2,
            GridLocalDir::South => 3,
        };
        (local_y * CHUNK_GRID_SUBDIVISIONS + local_x) as usize * 4 + dir_offset
    }
}


/// The main terrain grid resource that holds all the terrain chunks and their respective tiles.
#[derive(Resource, Default)]
pub struct Grid {
    chunks: HashMap<GridChunkPos, Chunk>
}
impl Grid {
    // Methods to manage entities in the grid
    /// Sets the entity at the specified chunk and local tile position in the grid.
    pub fn set_entity_at(&mut self, chunk_pos: GridChunkPos, local_pos: GridLocalPos, entity: Entity) {
        let chunk = self.chunks.entry(chunk_pos).or_insert_with(|| Chunk::new(chunk_pos));
        chunk.set_entity_at(local_pos, entity);
    }
    /// Gets the entity at the specified chunk and local tile position in the grid, if it exists.
    pub fn get_entity_at(&self, chunk_pos: GridChunkPos, local_pos: GridLocalPos) -> Option<Entity> {
        self.chunks.get(&chunk_pos).and_then(|chunk| chunk.get_entity_at(local_pos))
    }
    /// Clears the entity at the specified chunk and local tile position in the grid.
    pub fn clear_entity_at(&mut self, chunk_pos: GridChunkPos, local_pos: GridLocalPos) {
        if let Some(chunk) = self.chunks.get_mut(&chunk_pos) {
            chunk.clear_entity_at(local_pos);
        }
    }
    /// Clears all entities.
    pub fn clear_all(&mut self) {
        self.chunks.clear();
    }


    // Methods to convert between world positions and chunk/local positions
    /// Gets the nearest chunk and local tile position for a given world position.
    /// Note that this does not check if the chunk actually exists in the grid.
    pub fn world_to_pos(world_pos: Vec2) -> GridPos {
        let chunk_pos = Self::pos_to_chunk(world_pos);
        let local_pos = Chunk::world_to_local(world_pos);
        (chunk_pos, local_pos)
    }
    /// Gets the nearest chunk and tile position (without subtile direction) for a given world position.
    pub fn world_to_tile_pos(world_pos: Vec2) -> GridTilePos {
        let (chunk_pos, local_pos) = Self::world_to_pos(world_pos);
        (chunk_pos, (local_pos.0, local_pos.1))
    }
    /// Gets the nearest chunk position for a given world position.
    /// Note that this does not check if the chunk actually exists in the grid.
    pub fn pos_to_chunk(world_pos: Vec2) -> GridChunkPos {
        let half_chunk = CHUNK_SIZE * 0.5;
        ChunkPos::new(
            (world_pos.x + half_chunk).div_euclid(CHUNK_SIZE) as i32,
            (world_pos.y + half_chunk).div_euclid(CHUNK_SIZE) as i32,
        )
    }
    /// Gets the world position for a given chunk and local tile position.
    pub fn pos_to_world(chunk_pos: GridChunkPos, local_pos: GridLocalPos) -> Vec2 {
        Chunk::local_to_world(chunk_pos, local_pos)
    }
    /// Gets the world position of a tile center for a given chunk and tile position.
    pub fn tile_pos_to_world(chunk_pos: GridChunkPos, local_pos: GridTileLocalPos) -> Vec2 {
        let (local_x, local_y) = local_pos;
        let cell_size = CHUNK_SIZE / CHUNK_GRID_SUBDIVISIONS as f32;
        let half_cell = cell_size * 0.5;
        let half_chunk = CHUNK_SIZE * 0.5;
        Vec2::new(
            chunk_pos.x as f32 * CHUNK_SIZE + local_x as f32 * cell_size + half_cell - half_chunk,
            chunk_pos.y as f32 * CHUNK_SIZE + local_y as f32 * cell_size + half_cell - half_chunk,
        )
    }

    // Helper methods
    /// Gets the size of a single tile in world units.
    pub fn tile_size() -> f32 {
        CHUNK_SIZE / CHUNK_GRID_SUBDIVISIONS as f32
    }
    pub fn get_chunks(&self) -> impl Iterator<Item = (&GridChunkPos, &Chunk)> {
        self.chunks.iter()
    }
    pub fn get_chunks_mut(&mut self) -> impl Iterator<Item = (&GridChunkPos, &mut Chunk)> {
        self.chunks.iter_mut()
    }
}
