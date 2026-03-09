use bevy::prelude::*;

pub mod grid;
use grid::{render_grid, init_grid_cache};

pub(crate) struct RenderPlugin;
impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, init_grid_cache)
            .add_systems(Update, render_grid);
    }
}
