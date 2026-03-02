use bevy::prelude::*;

use crate::render::{TerrainRenderPlugin, renderer::TerrainRenderer};

mod render;

pub mod prelude {
    pub use super::TerrainPlugin;
}

pub struct TerrainPlugin;
impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(TerrainRenderPlugin)
            .add_systems(Startup, init);
    }
}

fn init(mut commands: Commands, assets_server: Res<AssetServer>) {
    commands.spawn((
        Name::new("Terrain"),
        TerrainRenderer::new("tests/terrain", 1, &assets_server)
    ));
}
