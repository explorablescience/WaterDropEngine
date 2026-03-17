use bevy::prelude::*;

mod ecs;
mod framedata;

pub struct PanelsPlugin;
impl Plugin for PanelsPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(ecs::UIEcsPanelPlugin)
            .add_plugins(framedata::UIFrameDataPlugin);
    }
}
