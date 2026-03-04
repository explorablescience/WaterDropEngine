use std::collections::HashSet;

use wde_terrain::prelude::*;
use wde_renderer::prelude::*;
use bevy::prelude::*;

use crate::{paint::{brush::PaintCommand, paint_manager::PaintManager}, processor::{compute::{computepass::apply_paint_compute, pipeline::{GpuPaintComputePipeline, PaintComputePipeline, PaintComputePipelineAsset}}, resources::{commands_buffer::ComputeCommandsBufferPlugin}}};

mod resources;
mod compute;

/// Stores the extracted paint commands and the dirty chunks that need to be updated
#[derive(Resource, Default)]
pub struct ExtractedPaintCommands {
    pub commands: Vec<PaintCommand>,
    pub dirty_chunks: Option<HashSet<TilePos>>
}

pub struct PaintProcessorPlugin;
impl Plugin for PaintProcessorPlugin {
    fn build(&self, app: &mut App) {
        app.get_sub_app_mut(RenderApp).unwrap()
            .init_resource::<ExtractedPaintCommands>()
            .add_systems(Extract, extract_paint_commands);

        // Add the resources plugins
        app
            .add_plugins(ComputeCommandsBufferPlugin);

        // Add the pipelines
        app
            .init_asset::<PaintComputePipelineAsset>()
            .add_plugins(RenderAssetsPlugin::<GpuPaintComputePipeline>::default());

        // Add the render pass
        app.get_sub_app_mut(RenderApp).unwrap()
            .add_systems(Render, apply_paint_compute.in_set(RenderSet::Render));
    }

    fn finish(&self, app: &mut App) {
        // Create the pipeline
        let pipeline = app.world_mut()
            .get_resource::<AssetServer>().unwrap().add(PaintComputePipelineAsset);
        app.get_sub_app_mut(RenderApp).unwrap().world_mut().spawn(PaintComputePipeline(pipeline));
    }
}

// Extracts paint commands from the PaintManager
fn extract_paint_commands(
    paint_manager: ExtractWorld<Res<PaintManager>>,
    mut extracted_commands: ResMut<ExtractedPaintCommands>,
) {
    extracted_commands.commands.extend(paint_manager.commands.clone().unwrap_or_default());
    if let Some(ref mut dirty_chunks) = extracted_commands.dirty_chunks {
        dirty_chunks.extend(paint_manager.commands_chunks.clone().unwrap_or_default());
    } else {
        extracted_commands.dirty_chunks = Some(paint_manager.commands_chunks.clone().unwrap_or_default());
    }
}
