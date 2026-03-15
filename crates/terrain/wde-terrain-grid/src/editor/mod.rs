use bevy::prelude::*;

use crate::editor::ui::{PlacementUI, handle_placement_tool, init_placement, init_ui, placement_system_ui};

pub mod ui;

pub struct EditorPlugin;
impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<PlacementUI>()
            .add_systems(Startup, (init_placement, init_ui))
            .add_systems(Update, (placement_system_ui, handle_placement_tool));
    }
}
