use bevy::prelude::*;

use crate::ui::EngineUiSet;

pub(crate) mod entities_edit_components;
pub(crate) mod entities_list;

pub struct UIEcsPanelPlugin;
impl Plugin for UIEcsPanelPlugin {
    fn build(&self, app: &mut App) {
        // Entities
        app.init_resource::<entities_edit_components::SelectedEntity>()
            .add_systems(
                Update,
                (
                    entities_list::draw_entities_panel,
                    entities_edit_components::draw_selected_entity_components_panel
                )
                    .in_set(EngineUiSet)
            );
    }
}
