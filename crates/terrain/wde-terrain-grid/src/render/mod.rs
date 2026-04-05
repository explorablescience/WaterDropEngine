use bevy::prelude::*;

pub mod ghost;
pub mod grid;

pub(crate) struct RenderPlugin;
impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(grid::TerrainGridPassesPlugin)
            .add_plugins(ghost::GhostPlugin);
    }
}
