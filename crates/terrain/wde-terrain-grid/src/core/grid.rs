use std::collections::HashMap;
use bevy::prelude::*;
use wde_terrain::prelude::CHUNK_SIZE;

/// The number of subdivisions per chunk in the grid system (number of grid cells per chunk).
pub const CHUNK_GRID_SUBDIVISIONS: u32 = 16;

/// The position of a chunk in the grid, represented as (x, z) coordinates.
pub type GridChunkPos = IVec2;
/// The local position of a tile within a chunk.
pub type GridLocalPos = (u32, u32, GridLocalDir);
/// The local position of a tile within a square tile.
pub enum GridLocalDir {
    NorthEast,
    NorthWest,
    SouthEast,
    SouthWest
}

/// The main terrain grid resource that holds all the terrain chunks and their respective tiles.
#[derive(Resource, Default)]
pub struct Grid {
    /// List of chunks that contain data.
    chunks: HashMap<GridChunkPos, Chunk>
}
impl Grid {
    /// Initializes the terrain grid by creating the necessary chunks based on the specified terrain size and chunk size.
    /// 
    /// # Arguments
    /// * `terrain_size` - The total size of the terrain in world units (e.g., [4000.0, 1000.0, 4000.0])
    /// 
    /// # Returns
    /// A `Grid` resource containing the initialized terrain chunks with their respective positions and empty tile lists.
    pub fn init(&mut self, terrain_size: Vec3) {
        let mut chunks = HashMap::new();
        let chunk_count_x = (terrain_size.x / CHUNK_SIZE[0]).ceil() as i32;
        let chunk_count_z = (terrain_size.z / CHUNK_SIZE[2]).ceil() as i32;

        for x in 0..chunk_count_x {
            for z in 0..chunk_count_z {
                // Keep chunk coordinates centered around origin to match terrain chunk indexing.
                let pos = GridChunkPos::new(x - (chunk_count_x / 2), z - (chunk_count_z / 2));
                chunks.insert(pos, Chunk::new(pos));
            }
        }

        self.chunks = chunks;
    }

    /// Gets a reference to the entity of the tile at the specified world position.
    /// 
    /// # Arguments
    /// * `world_pos` - The world position to query (e.g., [250.0, 0.0, 250.0])
    /// 
    /// # Returns
    /// An option containing a reference to the Entity of the tile if it exists, or None if the tile does not exist or is out of bounds.
    pub fn get_entity_at(&self, world_pos: Vec3) -> Option<Entity> {
        let (chunk_pos, local_pos) = Self::get_tile_position(world_pos)?;
        let chunk = self.chunks.get(&chunk_pos)?;
        chunk.get_local(local_pos)
    }

    /// Sets a tile in the grid at the specified world position and with the given Entity.
    /// 
    /// # Arguments
    /// * `world_pos` - The world position where the tile should be added (e.g., [250.0, 0.0, 250.0])
    /// * `entity` - The Entity of the tile to be added to the grid.
    pub fn set_entity_at(&mut self, world_pos: Vec3, entity: Entity) {
        let (chunk_pos, local_pos) = match Self::get_tile_position(world_pos) {
            Some(pos) => pos,
            None => // Create the necessary chunk if it doesn't exist
                Self::get_tile_position(world_pos).unwrap_or_else(|| {
                    let chunk_x = (world_pos.x / CHUNK_SIZE[0]).round() as i32;
                    let chunk_z = (world_pos.z / CHUNK_SIZE[2]).round() as i32;
                    let chunk_pos = GridChunkPos::new(chunk_x, chunk_z);
                    self.chunks.entry(chunk_pos).or_insert_with(|| Chunk::new(chunk_pos));
                    Self::get_tile_position(world_pos).expect("Chunk should have been created")
                })
        };
        Self::set_entity_at_chunk_local(self, chunk_pos, local_pos, entity);
    }

    /// Sets a tile in the grid at the specified chunk and local position with the given Entity.
    /// 
    /// # Arguments
    /// * `chunk_pos` - The position of the chunk in the grid (e.g., [0, 0] for the chunk at the origin)
    /// * `local_pos` - The local position of the tile within the chunk, represented as (x, z, direction)
    /// * `entity` - The Entity of the tile to be added to the grid.
    pub fn set_entity_at_chunk_local(&mut self, chunk_pos: GridChunkPos, local_pos: GridLocalPos, entity: Entity) {
        // Get or create the chunk at the specified position
        let chunk = self.chunks.entry(chunk_pos).or_insert_with(|| Chunk::new(chunk_pos));
        // Set the tile entity at the local position within the chunk
        let (x, z, dir) = local_pos;
        let dir_offset = match dir {
            GridLocalDir::NorthEast => 0,
            GridLocalDir::NorthWest => 1,
            GridLocalDir::SouthEast => 2,
            GridLocalDir::SouthWest => 3
        };
        let idx = ((z * CHUNK_GRID_SUBDIVISIONS + x) * 4 + dir_offset) as usize;
        if idx < chunk.tiles.len() {
            chunk.tiles[idx] = Some(entity);
        }
    }


    /// Gets an iterator over all chunks in the grid.
    pub fn get_chunks(&self) -> impl Iterator<Item = (&GridChunkPos, &Chunk)> {
        self.chunks.iter()
    }

    /// Helper function to calculate the chunk position and local tile position based on the world position.
    /// 
    /// # Arguments
    /// * `world_pos` - The world position to convert (e.g., [250.0, 0.0, 250.0])
    /// 
    /// # Returns
    /// An option containing a tuple of the chunk position (ChunkPos) and local tile position (TileLocalPos) if the world position is within the terrain bounds, or None if the position is out of bounds.
    fn get_tile_position(world_pos: Vec3) -> Option<(GridChunkPos, GridLocalPos)> {
        // Chunk coordinates represent chunk centers (0, 0 is centered on world origin).
        let chunk_x = (world_pos.x / CHUNK_SIZE[0]).round() as i32;
        let chunk_z = (world_pos.z / CHUNK_SIZE[2]).round() as i32;
        let chunk_pos = GridChunkPos::new(chunk_x, chunk_z);

        let cell_w = CHUNK_SIZE[0] / CHUNK_GRID_SUBDIVISIONS as f32;
        let cell_d = CHUNK_SIZE[2] / CHUNK_GRID_SUBDIVISIONS as f32;
        let chunk_min_x = (chunk_x as f32 * CHUNK_SIZE[0]) - (CHUNK_SIZE[0] * 0.5);
        let chunk_min_z = (chunk_z as f32 * CHUNK_SIZE[2]) - (CHUNK_SIZE[2] * 0.5);

        // Convert world position into chunk-local space [0, chunk_size).
        let local_xf = (world_pos.x - chunk_min_x).clamp(0.0, CHUNK_SIZE[0] - f32::EPSILON);
        let local_zf = (world_pos.z - chunk_min_z).clamp(0.0, CHUNK_SIZE[2] - f32::EPSILON);

        let local_x = (local_xf / cell_w).floor() as u32;
        let local_z = (local_zf / cell_d).floor() as u32;

        let in_cell_x = local_xf - (local_x as f32 * cell_w);
        let in_cell_z = local_zf - (local_z as f32 * cell_d);
        let local_dir = if in_cell_x < (cell_w * 0.5) {
            if in_cell_z < (cell_d * 0.5) {
                GridLocalDir::NorthWest
            } else {
                GridLocalDir::SouthWest
            }
        } else if in_cell_z < (cell_d * 0.5) {
            GridLocalDir::NorthEast
        } else {
            GridLocalDir::SouthEast
        };
        let local_pos = (local_x, local_z, local_dir);

        Some((chunk_pos, local_pos))
    }
}


/// A terrain chunk that contains a list of terrain tiles.
#[derive(Default)]
pub struct Chunk {
    /// The position of the chunk in the grid.
    pos: GridChunkPos,
    /// List of terrain tiles that belong to this chunk.
    /// The tile is None if it contains no data. If it is Some, the Entity should have a TerrainTileData component with the corresponding data.
    pub tiles: Vec<Option<Entity>>
}
impl Chunk {
    /// Creates a new terrain chunk with the specified position and an empty tile list.
    /// 
    /// # Arguments
    /// * `pos` - The position of the chunk in the grid (e.g., [0, 0] for the chunk at the origin)
    /// 
    /// # Returns
    /// A `Chunk` instance with the specified position and an empty tile list.
    fn new(pos: GridChunkPos) -> Self {
        let tiles = vec![None; (CHUNK_GRID_SUBDIVISIONS * CHUNK_GRID_SUBDIVISIONS * 4) as usize];
        Chunk { pos, tiles }
    }

    /// Gets the Entity of the tile at the specified local position within the chunk.
    /// 
    /// # Arguments
    /// * `local_pos` - The local position of the tile within the chunk, represented as (x, z, direction)
    /// 
    /// # Returns
    /// An option containing the Entity of the tile if it exists, or None if the tile does not exist or is out of bounds.
    pub fn get_local(&self, local_pos: GridLocalPos) -> Option<Entity> {
        let (x, z, dir) = local_pos;
        let dir_offset = match dir {
            GridLocalDir::NorthEast => 0,
            GridLocalDir::NorthWest => 1,
            GridLocalDir::SouthEast => 2,
            GridLocalDir::SouthWest => 3
        };
        let idx = ((z * CHUNK_GRID_SUBDIVISIONS + x) * 4 + dir_offset) as usize;
        *self.tiles.get(idx)?
    }

    /// Gets a reference to the tiles vector in this chunk.
    pub fn get_tiles(&self) -> &[Option<Entity>] {
        &self.tiles
    }
}


