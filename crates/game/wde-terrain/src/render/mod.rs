use bevy::prelude::*;
use wde_renderer::core::RenderApp;

use crate::render::{materials::{TerrainMaterialsPlugin, TerrainMaterialArrays}, passes::TerrainRenderFeaturesPlugin};

pub mod renderer;
mod passes;
mod materials;

pub struct TerrainRenderPlugin;
impl Plugin for TerrainRenderPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(TerrainRenderFeaturesPlugin)
            .add_plugins(TerrainMaterialsPlugin);

        // Spawn the terrain
        app.get_sub_app_mut(RenderApp).unwrap()
            .init_resource::<TerrainMaterialArrays>();
    }
}
