use bevy::prelude::*;

use crate::core::placement_config::PlacementConfigEntry;

mod place;
mod ui;

pub(crate) struct PlacementPlugin;
impl Plugin for PlacementPlugin {
    fn build(&self, app: &mut App) {
        let manager = TerrainPlacementManager::new(app.world_mut().commands());
        app.insert_resource(manager)
            .add_systems(Update, (ui::show_ui, place::place_update));
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) enum TerrainPlacementMode {
    #[default]
    None,
    Place,
    Remove
}

/// Resource to manage the state of the placement tool
#[derive(Resource)]
pub(crate) struct TerrainPlacementManager {
    pub mode: TerrainPlacementMode,
    pub entity: Entity,

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
            place_selected_entry_label: None
        }
    }
}
