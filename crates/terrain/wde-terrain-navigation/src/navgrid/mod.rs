use bevy::prelude::*;

mod debug;
mod graph;

pub(crate) use graph::rebuild_nav_grid;
pub use graph::{CONTROL_ROOM_ROLE, NavBase, NavBuilding, NavGrid, NavNeighbor};

pub struct NavMapPlugin;
impl Plugin for NavMapPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(debug::NavMapDebugPlugin)
            .init_resource::<NavGrid>()
            .add_systems(Update, graph::rebuild_nav_grid);
    }
}
