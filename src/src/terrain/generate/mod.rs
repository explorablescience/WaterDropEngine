use bevy::prelude::*;
use compute_core::MCComputeCorePoints;
use compute_pipeline::{GpuMCComputePipelineGenerate, MCComputePipelineGenerate, MCComputePipelineGenerateAsset};
use wde_render::{assets::RenderAssetsPlugin, core::{Render, RenderApp, RenderSet}};

mod compute_pipeline;
mod compute_core;

pub struct MCGeneratePlugin;
impl Plugin for MCGeneratePlugin {
    fn build(&self, app: &mut App) {
        // Register the compute pipeline asset
        app
            .init_asset::<MCComputePipelineGenerateAsset>()
            .add_plugins(RenderAssetsPlugin::<GpuMCComputePipelineGenerate>::default());

        // Compute pass
        app.get_sub_app_mut(RenderApp).unwrap()
            .add_systems(Render, (
                MCComputeCorePoints::create_bind_groups.in_set(RenderSet::BindGroups),
                MCComputeCorePoints::compute.in_set(RenderSet::Process),
            ));
    }

    fn finish(&self, app: &mut App) {
        // Create the compute pipeline instance
        let pipeline = app.world_mut()
            .get_resource::<AssetServer>().unwrap().add(MCComputePipelineGenerateAsset);
        app.get_sub_app_mut(RenderApp).unwrap().world_mut()
            .spawn(MCComputePipelineGenerate(pipeline));
    }
}
