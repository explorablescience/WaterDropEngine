use bevy::prelude::*;

use crate::{
    core::entries::PlacementConfigEntry, placement::move_and_delete::MoveAndDeleteManager
};
use wde_terrain::prelude::*;

mod move_and_delete;
mod place;
mod ui;

pub(crate) struct PlacementPlugin;
impl Plugin for PlacementPlugin {
    fn build(&self, app: &mut App) {
        let manager = TerrainPlacementManager::new(app.world_mut().commands());
        app.insert_resource(manager)
            .add_systems(Startup, set_parent)
            .add_systems(
                Update,
                (
                    ui::show_ui,
                    (place::place_update, place::handle_new_entity_to_place).chain(),
                    move_and_delete::handle_over_material,
                    move_and_delete::handle_move_and_delete_update
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
    pub place_selected_entry_label: Option<String>
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
