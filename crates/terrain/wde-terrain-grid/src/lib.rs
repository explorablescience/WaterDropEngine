use bevy::prelude::*;

use crate::{core::{CorePlugin, grid::Grid, grid_entity::{GridEntity, GridEntityRotation}}, render::RenderPlugin};

mod core;
mod render;

pub mod prelude {
    pub use super::TerrainGridPlugin;
}

pub struct TerrainGridPlugin;
impl Plugin for TerrainGridPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(CorePlugin)
            .add_plugins(RenderPlugin)
            .add_systems(Startup, init);
    }
}

fn init(mut commands: Commands, mut grid: ResMut<Grid>) {
    // Create a simple empty terrain
    let terrain_size = Vec3::new(4000.0, 1000.0, 4000.0);
    grid.init(terrain_size);

    // Add a dummy entity to test the grid system
    let center = Vec3::new(250.0, 0.0, 250.0);
    let size = Vec2::new(20.0, 20.0);
    let rotation = GridEntityRotation::R0;
    let footprint = GridEntity { center, size, rotation };
    let entity = commands.spawn(footprint.clone()).id();

    // For now, notify it "by hand" to the grid
    let occupied_tiles = footprint.get_occupied_tiles();
    for (chunk_pos, local_pos) in occupied_tiles {
        grid.set_entity_at_chunk_local(chunk_pos, local_pos, entity);
    }
}
