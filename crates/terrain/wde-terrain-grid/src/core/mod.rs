use bevy::prelude::*;

use crate::core::{grid::Grid, placement_config::PlacementPlugin};

pub mod grid;
pub mod grid_entity;
pub mod placement_config;

pub(crate) struct CorePlugin;
impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Grid>().add_plugins(PlacementPlugin);
    }
}
