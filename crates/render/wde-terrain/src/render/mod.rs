use bevy::prelude::*;

use crate::render::passes::TerrainRenderFeaturesPlugin;

mod passes;

pub struct TerrainRenderPlugin;
impl Plugin for TerrainRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TerrainRenderFeaturesPlugin);
    }
}
