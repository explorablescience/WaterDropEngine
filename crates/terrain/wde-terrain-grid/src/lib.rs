//! Grid placement of buildings on the terrain.
//!
//! The terrain is split into square tiles of [`TILE_SIZE`](core::grid::TILE_SIZE) world units.
//! The [`Grid`](prelude::Grid) resource knows which placed entity covers each tile.
//!
//! # Placeable entries
//! What can be placed is data-driven: `res/core/config/placement.json` lists the entries
//! ([`PlacementConfigEntry`](prelude::PlacementConfigEntry)) with a label, a menu category and a
//! glTF model. Their shape comes from `%P%` properties of the model:
//! - `Footprint`: extent in tiles.
//! - `AnchorIn*` / `AnchorOut*`: connection points. Two placed buildings are linked where anchors
//!   of the same kind meet: `AnchorIn` for pressurized links
//!   ([`LinkKind::Tunnel`](prelude::LinkKind)), `AnchorOut` for outdoor ones
//!   ([`LinkKind::Path`](prelude::LinkKind)).
//! - `Interior`: disc of floor where modules (`"placement": "module"` entries) can be placed.
//!
//! Entries can also be network pieces drawn with the line tool (`"network"`), have a cutaway
//! model without roof (`"model_cut"`), and carry a free `"role"` string that the engine never
//! interprets, for the game to give it a meaning.
//!
//! # Placing
//! Buildings are placed with the tools of the `Terrain/Placement` editor window, or of a game UI
//! sending [`PlacementCommand`](prelude::PlacementCommand) messages. From code, use the
//! [`PlacementQueue`](prelude::PlacementQueue):
//! ```rust,ignore
//! fn spawn_base(mut queue: ResMut<PlacementQueue>) {
//!     queue.place("Dome Medium", Vec2::new(1.0, 1.0), GridRotation::R0);
//!     // A module: placed inside the dome once it is there
//!     queue.place("Garden", Vec2::new(1.0, 1.0), GridRotation::R0);
//! }
//! ```
//! A placed building is an entity with a [`GridEntity`](prelude::GridEntity) (its entry, position
//! and rotation), its meshes and a collider in `BUILDINGS_COLLIDER_GROUP`. A module also has an
//! [`InteriorModule`](prelude::InteriorModule) pointing to its host building, and its collider is
//! in `MODULES_COLLIDER_GROUP`.
//!
//! # Building graph
//! When a building is placed or removed, the systems in
//! [`BuildingGraphSet`](prelude::BuildingGraphSet) update the
//! [`BuildingConnections`](prelude::BuildingConnections) of each building (its connected anchors,
//! choosing its model variant) and the [`BuildingGraph`](prelude::BuildingGraph) resource: the
//! links between buildings, and the groups of linked buildings. Modules are not part of it: they
//! follow their host.
//! ```rust,ignore
//! fn list_groups(graph: Res<BuildingGraph>) {
//!     for group in graph.groups() {
//!         info!("{} linked buildings", group.len());
//!     }
//! }
//! app.add_systems(Update, list_groups.after(BuildingGraphSet));
//! ```
use bevy::prelude::*;

use crate::{core::CorePlugin, placement::PlacementPlugin, render::RenderPlugin};

pub mod core;
mod placement;
mod render;

#[doc(hidden)]
pub mod prelude {
    pub use super::core::entries::{
        InteriorArea, NetworkPiece, PlacementConfig, PlacementConfigEntry
    };
    pub use super::core::grid::{Grid, GridEntityEvent, GridTilePos};
    pub use super::core::grid_entity::{GridEntity, GridRotation};
    pub use super::placement::{
        AnchorLink, BuildingConnections, BuildingCutaway, BuildingGraph, BuildingGraphSet,
        BuildingLink, InteriorModule, LinkKind, PlacementCommand, PlacementInputCapture,
        PlacementQueue, PlacementTool, anchor_links, connected_anchors
    };
}

pub struct TerrainGridPlugin;
impl Plugin for TerrainGridPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RenderPlugin)
            .add_plugins((PlacementPlugin, CorePlugin));
    }
}
