use bevy::prelude::*;

use crate::core::{grid::Grid, placement_config::PlacementPlugin};
use wde_terrain::prelude::*;

pub mod grid;
pub mod grid_entity;
pub mod placement_config;

pub(crate) struct CorePlugin;
impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Grid>()
            .add_plugins(PlacementPlugin)
            .add_systems(Startup, set_parent);
    }
}

fn set_parent(
    mut commands: Commands,
    terrain: Single<Entity, With<Terrain>>,
    mut grid: ResMut<Grid>
) {
    let parent = commands
        .spawn((
            Name::new("Terrain Entities"),
            Transform::default(),
            ChildOf(*terrain)
        ))
        .id();
    grid.set_parent(parent);
}
