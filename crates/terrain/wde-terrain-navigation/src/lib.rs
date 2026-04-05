use bevy::prelude::*;

pub mod prelude {
    pub use super::TerrainNavigationPlugin;
    pub use super::TerrainNavigationProvider;
}

pub struct TerrainNavigationPlugin;
impl Plugin for TerrainNavigationPlugin {
    fn build(&self, _app: &mut App) {
        // app.add_systems(Startup, setup);
    }
}

pub struct TerrainNavigationProvider;
