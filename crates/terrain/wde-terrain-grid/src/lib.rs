//! Terrain grid plugin for WaterDropEngine.
//!
//! WIP.
use bevy::prelude::*;

use crate::{
    core::{CorePlugin, grid::Grid},
    placement::PlacementPlugin,
    render::RenderPlugin
};

mod core;
mod placement;
mod render;

#[doc(hidden)]
pub mod prelude {
    pub use super::core::grid::{Grid, GridLocalDir, GridTilePos};
    pub use super::core::grid_entity::{GridEntity, GridRotation};
}

pub struct TerrainGridPlugin;
impl Plugin for TerrainGridPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((PlacementPlugin, CorePlugin, RenderPlugin))
            .init_resource::<Grid>();
    }
}
