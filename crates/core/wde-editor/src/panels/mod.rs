use bevy::prelude::*;

mod assets;
mod ecs;
mod framedata;
mod logs;
mod pipeline;
mod pipeline_plot;
mod pipeline_status;
mod profiler;

pub use framedata::{FrameDataAverages, FrameDataOverlayConfig};

pub struct PanelsPlugin;
impl Plugin for PanelsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(assets::AssetsPanelPlugin)
            .add_plugins(ecs::UIEcsPanelPlugin)
            .add_plugins(framedata::UIFrameDataPlugin)
            .add_plugins(logs::LogsPanelPlugin)
            .add_plugins(profiler::ProfilerPlugin)
            .add_plugins(pipeline::PipelinePanelPlugin)
            .add_plugins(pipeline_status::PipelineStatusPanelPlugin);
    }
}
