use bevy::prelude::*;
use wde_pbr::prelude::PbrModel;

use crate::editor::{placement::{handle_placement_tool, init_placement}, ui::*};

mod ui;
mod placement;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PlacementTool {
    #[default]
    Place
}

#[derive(Resource)]
pub struct PlacementUI {
    pub enabled: bool,
    pub selected_tools: PlacementTool,

    pub placement_show_entity: bool,
    pub placement_entity: Option<Entity>,
    pub placement_entity_has_model: bool,
    pub placement_extent: UVec2,
    pub placement_model: Option<PbrModel>,
}
impl Default for PlacementUI {
    fn default() -> Self {
        PlacementUI {
            enabled: false,
            selected_tools: PlacementTool::default(),

            placement_show_entity: false,
            placement_entity: None,
            placement_entity_has_model: false,
            placement_extent: UVec2::new(2, 2),
            placement_model: None
        }
    }
}

pub struct EditorPlugin;
impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<PlacementUI>()
            .add_systems(Startup, (init_placement, init_ui))
            .add_systems(Update, (show_ui, handle_placement_tool));
    }
}
