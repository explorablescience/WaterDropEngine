use bevy::prelude::*;

use crate::{physics::{TerrainPhysicsPlugin, collider::TerrainCollider}, render::{TerrainRenderPlugin, renderer::TerrainRenderer}};

mod physics;
mod render;
mod manager;

pub mod prelude {
    pub use super::TerrainPlugin;
}

pub struct TerrainPlugin;
impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(TerrainRenderPlugin)
            .add_plugins(TerrainPhysicsPlugin)
            .add_systems(Startup, init);
    }
}

fn init(mut commands: Commands, assets_server: Res<AssetServer>) {
    commands.spawn((
        TerrainCollider::new("tests/terrain", 4),
        TerrainRenderer::new("tests/terrain", 4, &assets_server)
    ));
}
