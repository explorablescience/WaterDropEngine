//! Navigation of walking entities over the terrain and through placed buildings.
//!
//! Paths are searched with A* over the outdoor grid tiles, and over walk cells inside buildings
//! baked from their models (geometry blocks, `%P%Door*` boxes are doors). Everything is read
//! from the placed buildings and their [`BuildingGraph`](wde_terrain_grid::prelude::BuildingGraph),
//! without any game-specific role:
//! - A building with tunnel anchors (`AnchorIn`) is pressurized. Pressurized buildings are walked
//!   between through their tunnel links.
//! - A pressurized building with path anchors (`AnchorOut`) too is an airlock: the only way
//!   between pressurized and outdoor places, with a stop for its cycle.
//! - Outdoor network pieces (e.g. paths) are walked on like the ground, faster. Steep ground is
//!   not walked on.
//!
//! Each step costs its walking time (see [`WalkSpeeds`](prelude::WalkSpeeds)), so walkers
//! prefer paths and tunnels.
//!
//! # Moving entities
//! Any entity with a `Transform` can be sent somewhere through the [`TerrainNavigator`]. Its
//! optional [`NavAgent`](prelude::NavAgent) sets its body radius and speed, and
//! [`Indoors`](prelude::Indoors) is kept up to date when it goes through an airlock:
//! ```rust,ignore
//! fn send_colonists(
//!     mut navigator: ResMut<TerrainNavigator>,
//!     colonists: Query<Entity, With<Colonist>>
//! ) {
//!     // Each one gets its own standing spot around the destination
//!     navigator.add_group(&colonists, Vec3::new(10.0, 0.0, 4.0));
//! }
//! ```
//! Paths are replanned when buildings are placed or removed. The `Debug/Navigation` editor window
//! shows the building graph, the paths and the walkable cells.
use bevy::prelude::*;

mod debug;
mod navigation;

#[doc(hidden)]
pub mod prelude {
    pub use super::TerrainNavigator;
    pub use super::navigation::{Indoors, NavAgent, WalkSpeeds};
}

pub struct TerrainNavigationPlugin;
impl Plugin for TerrainNavigationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((navigation::NavigationPlugin, debug::NavDebugPlugin))
            .init_resource::<TerrainNavigator>();
    }
}

/// Requests movements: entities walk to their destination along a path through the colony
/// (outdoor tiles, paths, airlocks, tunnels and buildings).
#[derive(Resource, Default)]
pub struct TerrainNavigator {
    new_entities: Vec<(Entity, Vec3)>,
    groups: Vec<(Vec<Entity>, Vec3)>,
    removed_entities: Vec<Entity>
}
impl TerrainNavigator {
    /// Sends `entity` toward the world position `destination`.
    pub fn add(&mut self, entity: Entity, destination: Vec3) {
        self.new_entities.push((entity, destination));
    }

    /// Sends `entities` toward `destination` together: each one gets its own standing spot
    /// around it, so that they do not pile up on the same point.
    pub fn add_group(&mut self, entities: impl IntoIterator<Item = Entity>, destination: Vec3) {
        self.groups
            .push((entities.into_iter().collect(), destination));
    }

    /// Stops `entity`.
    pub fn remove(&mut self, entity: Entity) {
        self.removed_entities.push(entity);
    }
}
