use std::collections::HashSet;

use bevy::prelude::*;
use wde_renderer::prelude::*;
use wde_terrain::prelude::*;
use wde_terrain::render::extractor::TerrainRenderSets;

use crate::{
    paint::{brush::PaintCommand, paint_manager::PaintManager},
    processor::{
        compute::{computepass::apply_paint_compute, pipeline::PaintComputePipeline},
        resources::commands_buffer::ComputeCommandsBufferPlugin
    }
};

mod compute;
mod resources;

/// Extracted paint commands forwarded to the render world each frame.
#[derive(Resource, Default)]
pub struct ExtractedPaintCommands {
    pub commands: Vec<PaintCommand>,
    pub dirty_chunks: Option<HashSet<ChunkPos>>
}

pub struct PaintProcessorPlugin;
impl Plugin for PaintProcessorPlugin {
    fn build(&self, app: &mut App) {
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .init_resource::<ExtractedPaintCommands>()
            .add_systems(Extract, extract_paint_commands);

        app.add_plugins(ComputeCommandsBufferPlugin)
            .add_plugins(RenderPipelineRegisterPlugin::<PaintComputePipeline>::default())
            // Queue heightmap readbacks for physics whenever a paint flush happens.
            .add_systems(Update, queue_physics_extraction);

        // Place apply_paint_compute in the TextureWrite set so that initiate_tile_readbacks
        // (which is in TerrainExtractorPlugin, ordered after TextureWrite) always runs
        // after the paint compute has submitted its GPU work.
        app.get_sub_app_mut(RenderApp).unwrap().add_systems(
            Render,
            apply_paint_compute
                .in_set(RenderSet::Render)
                .in_set(TerrainRenderSets::TextureWrite)
        );
    }
}

fn extract_paint_commands(
    paint_manager: ExtractWorld<Res<PaintManager>>,
    mut extracted: ResMut<ExtractedPaintCommands>
) {
    let Some((commands, chunks)) = paint_manager.to_extract.clone() else {
        return;
    };
    extracted.commands.extend(commands);
    extracted
        .dirty_chunks
        .get_or_insert_with(HashSet::new)
        .extend(chunks);
}

/// After each paint flush, queue heightmap GPU readbacks for the affected chunks so the
/// physics colliders can be rebuilt with the new heights.
fn queue_physics_extraction(
    paint_manager: Res<PaintManager>,
    mut extractor: ResMut<TerrainExtractor>
) {
    if let Some((_, dirty_chunks)) = &paint_manager.to_extract {
        for &chunk_pos in dirty_chunks {
            extractor.queue_tile_extraction(chunk_pos, 0, 0);
        }
    }
}
