use bevy::prelude::*;
use wde_renderer::prelude::*;

pub mod cursor_pos;
pub mod image_decoder;

pub struct TerrainUtilsPlugin;
impl Plugin for TerrainUtilsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<cursor_pos::TerrainCursorPos>()
            .add_systems(Update, cursor_pos::TerrainCursorPos::update);
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .init_resource::<cursor_pos::TerrainCursorPos>()
            .add_systems(Extract, cursor_pos::TerrainCursorPos::extract);
    }
}
