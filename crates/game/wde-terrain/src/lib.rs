use bevy::prelude::*;

use crate::{manager::Terrain, physics::TerrainPhysicsPlugin, render::{TerrainRenderPlugin, renderer::TerrainRenderer}};

pub(crate) mod manager;
pub(crate) mod render;
pub(crate) mod physics;
pub(crate) mod utils;

pub mod prelude {
    pub use super::TerrainPlugin;
    pub use crate::manager::Terrain;
}

pub struct TerrainPlugin;
impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        // Add the terrain plugin and its dependencies
        app
            .add_plugins(TerrainRenderPlugin)
            .add_plugins(TerrainPhysicsPlugin);

        // Add the terrain system
        app
            .add_systems(PostUpdate, Terrain::clear_dirty);

        // Test system
        app
            .add_systems(Startup, init);
    }
}

fn init(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Terrain::load("tests/terrain"),
        TerrainRenderer::new(&asset_server)
    ));
}
