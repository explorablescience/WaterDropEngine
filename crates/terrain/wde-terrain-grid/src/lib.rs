use bevy::prelude::*;

use crate::{core::{CorePlugin, grid::Grid}, editor::EditorPlugin, render::RenderPlugin};

mod core;
mod render;
mod editor;

pub mod prelude {
    pub use super::TerrainGridPlugin;
    pub use super::core::grid::{Grid, GridLocalDir, GridTilePos};
    pub use super::core::grid_entity::{GridEntity, GridRotation};
}

pub struct TerrainGridPlugin;
impl Plugin for TerrainGridPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(EditorPlugin)
            .add_plugins(CorePlugin)
            .add_plugins(RenderPlugin);

        app
            .init_resource::<Grid>();
    }
}
