use bevy::prelude::*;

use crate::{core::placement_config::PlacementConfigEntry, editor::{placement::*, ui::*}};

mod ui;
mod placement;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PlacementTool {
    #[default]
    Place
}
impl PlacementTool {
    pub fn iter() -> impl Iterator<Item = PlacementTool> {
        [PlacementTool::Place].into_iter()
    }
}

#[derive(Resource, Default)]
pub struct PlacementUI {
    pub enabled: bool,
    pub selected_tool: PlacementTool,

    pub placement_entry_label: Option<String>,
    pub placement_entry: Option<PlacementConfigEntry>,
    pub placement_entity: Option<Entity>
}

pub struct EditorPlugin;
impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<PlacementUI>()
            .add_systems(Startup, init_ui)
            .add_systems(Update, (show_ui, handle_placement_tool));
    }
}
