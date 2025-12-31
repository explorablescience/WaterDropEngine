use bevy::prelude::*;
use compute_pipeline::{GpuMCComputePipelineSpawn, MCComputePipelineSpawn, MCComputePipelineSpawnAsset};
use spawner::MarchingCubesSpawner;
use compute_core::MCComputePointsCore;
use wde_render::{assets::RenderAssetsPlugin, core::{Extract, Render, RenderApp, RenderSet}};

use super::{mc_chunk::{MCChunksListMain, MCChunksListRender}, mc_compute_main::MCTerrainNoiseParameters};

mod spawner;
mod compute_core;
mod compute_pipeline;

pub struct MCSpawnPlugin;
impl Plugin for MCSpawnPlugin {
    fn build(&self, app: &mut App) {
        // Register the compute pipeline asset
        app
            .init_asset::<MCComputePipelineSpawnAsset>()
            .add_plugins(RenderAssetsPlugin::<GpuMCComputePipelineSpawn>::default());

        // Add the chunk management systems
        app
            .init_resource::<MCChunksListMain>()
            .add_systems(Update, MarchingCubesSpawner::manage_chunks);

        // Compute pass
        app.get_sub_app_mut(RenderApp).unwrap()
            .init_resource::<MCTerrainNoiseParameters>()
            .init_resource::<MCChunksListRender>()
            .add_systems(Extract, MCComputePointsCore::extract)
            .add_systems(Render, (
                MCComputePointsCore::create_bind_groups.in_set(RenderSet::BindGroups),
                MCComputePointsCore::compute.in_set(RenderSet::Process)
            ));
    }

    fn finish(&self, app: &mut App) {
        // Create the compute pipeline instance
        let pipeline = app.world_mut()
            .get_resource::<AssetServer>().unwrap().add(MCComputePipelineSpawnAsset);
        app.get_sub_app_mut(RenderApp).unwrap().world_mut()
            .spawn(MCComputePipelineSpawn(pipeline));
    }
}
