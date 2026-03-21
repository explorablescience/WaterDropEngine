use bevy::prelude::*;

mod ecs;
mod framedata;
mod logs;
mod profiler;

pub struct PanelsPlugin;
impl Plugin for PanelsPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(ecs::UIEcsPanelPlugin)
            .add_plugins(framedata::UIFrameDataPlugin)
            .add_plugins(logs::LogsPanelPlugin)
            .add_plugins(profiler::ProfilerPlugin);
    }
}
