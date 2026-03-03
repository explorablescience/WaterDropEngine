use bevy::prelude::*;

use crate::render::{dependencies::BuffersPlugin, passes::TerrainPassesPlugin};

pub mod renderer;
pub mod dependencies;
mod passes;

pub struct TerrainRenderPlugin;
impl Plugin for TerrainRenderPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(BuffersPlugin)
            .add_plugins(TerrainPassesPlugin);
    }
}
