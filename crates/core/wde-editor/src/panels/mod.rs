use bevy::prelude::*;

mod ecs;
mod framedata;
mod logs;
mod profiler;
mod rendergraph;

pub use framedata::FrameDataOverlayConfig;

pub struct PanelsPlugin;
impl Plugin for PanelsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ecs::UIEcsPanelPlugin)
            .add_plugins(framedata::UIFrameDataPlugin)
            .add_plugins(logs::LogsPanelPlugin)
            .add_plugins(profiler::ProfilerPlugin)
            .add_plugins(rendergraph::RenderGraphPanelPlugin);
    }
}
