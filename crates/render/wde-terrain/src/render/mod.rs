use bevy::prelude::*;
use wde_renderer::core::{Render, RenderApp, RenderSet};

use crate::render::{passes::TerrainRenderFeaturesPlugin, terrain::Terrain};

mod passes;
mod terrain;

pub struct TerrainRenderPlugin;
impl Plugin for TerrainRenderPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(TerrainRenderFeaturesPlugin);

        // Spawn the terrain
        let terrain = Terrain::init(1, "tests/terrain", app.world().resource::<AssetServer>());
        app.get_sub_app_mut(RenderApp).unwrap()
            .insert_resource(terrain)
            .add_systems(Render, Terrain::build_bind_group.in_set(RenderSet::BindGroups).run_if(run_once));
    }
}
