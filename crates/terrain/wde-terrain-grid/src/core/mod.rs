use bevy::prelude::*;

use crate::core::{
    entries::PlacementPlugin,
    grid::{Grid, GridEntityEvent}
};
use wde_terrain::prelude::*;

pub mod entries;
pub mod grid;
pub mod grid_entity;

pub(crate) struct CorePlugin;
impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Grid>()
            .add_message::<GridEntityEvent>()
            .add_plugins(PlacementPlugin)
            .add_systems(Startup, set_parent)
            .add_systems(Update, handle_grid_entity_events);
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

fn handle_grid_entity_events(mut grid: ResMut<Grid>, mut events: MessageReader<GridEntityEvent>) {
    for event in events.read() {
        match event {
            GridEntityEvent::Placed {
                entity,
                grid_entity
            } => {
                grid.set_entity(grid_entity, *entity);
            }
            GridEntityEvent::Removed { entity } => {
                grid.remove_entity(*entity);
            }
        }
    }
}
