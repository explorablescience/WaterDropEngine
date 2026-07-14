//! Terrain grid plugin for WaterDropEngine.
//!
//! WIP.
use bevy::prelude::*;

use crate::{core::CorePlugin, placement::PlacementPlugin, render::RenderPlugin};

mod core;
mod placement;
mod render;

#[doc(hidden)]
pub mod prelude {
    pub use super::core::grid::{Grid, GridTilePos, GridEntityEvent};
    pub use super::core::grid_entity::{GridEntity, GridRotation};
}

pub struct TerrainGridPlugin;
impl Plugin for TerrainGridPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RenderPlugin)
            .add_plugins((PlacementPlugin, CorePlugin));
    }
}
