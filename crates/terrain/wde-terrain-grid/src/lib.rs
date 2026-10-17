//! Terrain grid plugin for WaterDropEngine.
//!
//! WIP.
use bevy::prelude::*;

use crate::{core::CorePlugin, placement::PlacementPlugin, render::RenderPlugin};

pub mod core;
mod placement;
mod render;

#[doc(hidden)]
pub mod prelude {
    pub use super::core::entries::InteriorArea;
    pub use super::core::grid::{Grid, GridEntityEvent, GridTilePos};
    pub use super::core::grid_entity::{GridEntity, GridRotation};
    pub use super::placement::{
        AnchorLink, BuildingConnections, BuildingCutaway, InteriorModule, LinkKind,
        PlacementInputCapture, PlacementQueue, anchor_links, connected_anchors
    };
}

pub struct TerrainGridPlugin;
impl Plugin for TerrainGridPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RenderPlugin)
            .add_plugins((PlacementPlugin, CorePlugin));
    }
}
