use bevy::prelude::*;

mod assets;
mod entities;

pub struct UIEcsPanelPlugin;
impl Plugin for UIEcsPanelPlugin {
    fn build(&self, app: &mut App) {
        // Entities
        app.init_resource::<entities::SelectedEntity>().add_systems(
            Update,
            (
                entities::draw_entities_panel,
                entities::draw_selected_entity_components_panel
            )
        );

        // Assets
        app.init_resource::<assets::AssetCatalog>()
            .add_systems(PostStartup, assets::init_asset_catalog)
            .add_systems(Update, assets::draw_assets_panel);
    }
}
