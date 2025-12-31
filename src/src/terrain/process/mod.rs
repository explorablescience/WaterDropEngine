use bevy::prelude::*;
use process_chunks::MCProcessTaskManager;
use wde_render::core::{Render, RenderApp, RenderSet};

mod process_chunks;

pub struct MCProcessPlugin;
impl Plugin for MCProcessPlugin {
    fn build(&self, app: &mut App) {
        app.get_sub_app_mut(RenderApp).unwrap()
            .add_systems(Render, (
                MCProcessTaskManager::process_chunks,
                MCProcessTaskManager::handle_tasks
            ).in_set(RenderSet::Process));
    }
}
