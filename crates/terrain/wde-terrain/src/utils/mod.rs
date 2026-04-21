use bevy::prelude::*;
use wde_renderer::prelude::*;

pub mod cursor_pos;
pub mod image_decoder;

pub struct TerrainUtilsPlugin;
impl Plugin for TerrainUtilsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<cursor_pos::TerrainCursorPos>()
            .add_plugins(ExtractResourcePlugin::<cursor_pos::TerrainCursorPos>::default())
            .add_systems(Update, cursor_pos::TerrainCursorPos::update);
    }
}
