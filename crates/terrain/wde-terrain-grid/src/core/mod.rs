use bevy::prelude::*;

use crate::core::grid::Grid;

pub mod grid_entity;
pub mod grid;

pub(crate) struct CorePlugin;
impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Grid>();
    }
}
