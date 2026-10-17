use bevy::prelude::*;

mod navgrid;
mod navigation;

pub mod prelude {
    pub use super::TerrainNavigator;
    pub use super::navgrid::{CONTROL_ROOM_ROLE, NavBase, NavBuilding, NavGrid, NavNeighbor};
    pub use super::navigation::{AIRLOCK_ROLE, Indoors, NavAgent, WalkSpeeds};
}

pub struct TerrainNavigationPlugin;
impl Plugin for TerrainNavigationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((navigation::NavigationPlugin, navgrid::NavMapPlugin))
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
