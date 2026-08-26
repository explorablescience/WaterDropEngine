use std::collections::HashMap;

use bevy::prelude::*;
use wde_terrain_grid::{
    core::grid::{CHUNK_GRID_SUBDIVISIONS, GridChunkPos, GridLocalPos},
    prelude::*
};

use crate::navgrid::NavMap;

pub struct NavMapExteriorPlugin;
impl Plugin for NavMapExteriorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, handle_grid_events);
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Hash, Copy)]
enum TileStatus {
    Empty,
    Building,
    Road
}

struct Chunk {
    tiles: Vec<TileStatus>
}
impl Default for Chunk {
    fn default() -> Self {
        Chunk {
            tiles: vec![
                TileStatus::Empty;
                (CHUNK_GRID_SUBDIVISIONS * CHUNK_GRID_SUBDIVISIONS * 4) as usize
            ]
        }
    }
}
impl Chunk {
    pub fn set(&mut self, local_pos: GridLocalPos, value: TileStatus) {
        let index = wde_terrain_grid::core::grid::Chunk::local_pos_to_index(local_pos);
        if let Some(tile) = self.tiles.get_mut(index) {
            *tile = value;
        }
    }
    pub fn get(&self, local_pos: GridLocalPos) -> TileStatus {
        let index = wde_terrain_grid::core::grid::Chunk::local_pos_to_index(local_pos);
        self.tiles.get(index).copied().unwrap_or(TileStatus::Empty)
    }
    pub fn remove(&mut self, local_pos: GridLocalPos) {
        let index = wde_terrain_grid::core::grid::Chunk::local_pos_to_index(local_pos);
        if let Some(tile) = self.tiles.get_mut(index) {
            *tile = TileStatus::Empty;
        }
    }
}

#[derive(Default)]
pub struct ExteriorNavMap {
    chunks: HashMap<GridChunkPos, Chunk>
}

fn handle_grid_events(
    mut grid_events: MessageReader<GridEntityEvent>,
    mut nav_map: ResMut<NavMap>
) {
    let _nav_map = &mut nav_map.exterior;
    for event in grid_events.read() {
        match event {
            GridEntityEvent::Placed {
                entity: _,
                grid_entity
            } => {
                let _entry = grid_entity.entry();
            }
            GridEntityEvent::Removed { entity: _ } => {}
        }
    }
}
