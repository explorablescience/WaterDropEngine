use std::collections::BTreeSet;

use bevy::prelude::*;
use wde_editor::prelude::*;

use crate::{
    core::entries::PlacementConfigEntry, placement::move_and_delete::MoveAndDeleteManager
};
use wde_terrain::prelude::*;

pub use commands::{PlacementCommand, PlacementTool};
pub use connections::{
    AnchorLink, BuildingConnections, BuildingGraph, BuildingGraphSet, BuildingLink, LinkKind,
    anchor_links, connected_anchors
};
pub use cutaway::BuildingCutaway;
pub use modules::InteriorModule;
pub use queue::PlacementQueue;

/// Who currently uses the left mouse button, so other click interactions (e.g. drag selection)
/// can stand by.
#[derive(Resource, Default, PartialEq, Eq)]
pub struct PlacementInputCapture {
    /// A placement tool is active: a building or line being placed, or the move / remove tools.
    pub tool: bool,
    /// The pointer is over a UI window.
    pub over_ui: bool
}
impl PlacementInputCapture {
    pub fn is_captured(&self) -> bool {
        self.tool || self.over_ui
    }
}

fn update_input_capture(
    manager: Res<TerrainPlacementManager>,
    ui: Option<Res<UIContext>>,
    mut capture: ResMut<PlacementInputCapture>
) {
    let tool = match manager.mode {
        TerrainPlacementMode::None => false,
        TerrainPlacementMode::Place => {
            manager.place_selected_entry.is_some() || manager.line_network.is_some()
        }
        TerrainPlacementMode::Move | TerrainPlacementMode::Remove => true
    };
    let over_ui = ui.is_some_and(|ui| ui.0.is_pointer_over_area());
    capture.set_if_neq(PlacementInputCapture { tool, over_ui });
}

mod commands;
mod connections;
mod cutaway;
mod line_tool;
mod modules;
mod move_and_delete;
mod place;
mod queue;
mod ui;

pub(crate) struct PlacementPlugin;
impl Plugin for PlacementPlugin {
    fn build(&self, app: &mut App) {
        let manager = TerrainPlacementManager::new(app.world_mut().commands());
        app.insert_resource(manager)
            .init_resource::<BuildingCutaway>()
            .init_resource::<BuildingGraph>()
            .init_resource::<PlacementInputCapture>()
            .init_resource::<PlacementQueue>()
            .init_resource::<PlacementTool>()
            .add_message::<PlacementCommand>()
            .add_systems(Startup, set_parent)
            .add_systems(
                Update,
                (
                    (
                        ui::show_ui,
                        commands::apply_placement_commands,
                        (place::place_update, place::handle_new_entity_to_place).chain(),
                        commands::update_placement_tool
                    )
                        .chain(),
                    move_and_delete::handle_over_material,
                    move_and_delete::handle_move_and_delete_update,
                    line_tool::line_tool_update,
                    update_input_capture,
                    queue::process_placement_queue,
                    modules::remove_orphan_modules,
                    (
                        cutaway::toggle_cutaway,
                        connections::update_connections.in_set(BuildingGraphSet),
                        cutaway::sync_building_variants
                    )
                        .chain()
                )
            );
    }

    fn finish(&self, app: &mut App) {
        let move_and_delete_manager =
            MoveAndDeleteManager::new(app.world().resource::<AssetServer>());
        app.insert_resource(move_and_delete_manager);
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) enum TerrainPlacementMode {
    /// Placement window closed, no tool active.
    #[default]
    None,
    Place,
    Move,
    Remove
}

/// Resource to manage the state of the placement tool
#[derive(Resource)]
pub(crate) struct TerrainPlacementManager {
    pub mode: TerrainPlacementMode,
    pub entity: Entity,
    pub new_entry_to_place: Option<PlacementConfigEntry>,

    pub place_selected_entry: Option<PlacementConfigEntry>,
    pub place_selected_entry_label: Option<String>,
    /// Category tab shown in the build menu.
    pub place_selected_category: Option<String>,
    /// Connected anchors the ghost parts were spawned for (preview of the placed variant).
    pub ghost_connections: Option<BTreeSet<String>>,
    /// Network drawn by the line tool, instead of placing a single building.
    pub line_network: Option<String>,
    pub line_tool: line_tool::LineToolState,
    /// Parent of the line tool preview ghosts.
    pub line_preview_root: Entity
}
impl TerrainPlacementManager {
    pub fn new(mut commands: Commands) -> Self {
        Self {
            mode: TerrainPlacementMode::None,
            entity: commands
                .spawn((
                    Name::new("Terrain Placement Tool"),
                    Transform::from_translation(Vec3::MIN)
                ))
                .id(),
            place_selected_entry: None,
            place_selected_entry_label: None,
            place_selected_category: None,
            ghost_connections: None,
            line_network: None,
            line_tool: Default::default(),
            line_preview_root: commands
                .spawn((Name::new("Terrain Line Tool Preview"), Transform::default()))
                .id(),
            new_entry_to_place: None
        }
    }
}

fn set_parent(
    mut commands: Commands,
    manager: Res<TerrainPlacementManager>,
    terrain: Single<Entity, With<Terrain>>
) {
    commands.entity(manager.entity).insert(ChildOf(*terrain));
}
