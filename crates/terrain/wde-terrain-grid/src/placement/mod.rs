use std::collections::BTreeSet;

use bevy::prelude::*;

use crate::{
    core::entries::PlacementConfigEntry, placement::move_and_delete::MoveAndDeleteManager
};
use wde_terrain::prelude::*;

pub use connections::{BuildingConnections, connected_anchors};
pub use cutaway::BuildingCutaway;

mod connections;
mod cutaway;
mod line_tool;
mod move_and_delete;
mod place;
mod ui;

pub(crate) struct PlacementPlugin;
impl Plugin for PlacementPlugin {
    fn build(&self, app: &mut App) {
        let manager = TerrainPlacementManager::new(app.world_mut().commands());
        app.insert_resource(manager)
            .init_resource::<BuildingCutaway>()
            .add_systems(Startup, set_parent)
            .add_systems(
                Update,
                (
                    ui::show_ui,
                    (place::place_update, place::handle_new_entity_to_place).chain(),
                    move_and_delete::handle_over_material,
                    move_and_delete::handle_move_and_delete_update,
                    line_tool::line_tool_update,
                    (
                        cutaway::toggle_cutaway,
                        connections::update_connections,
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
