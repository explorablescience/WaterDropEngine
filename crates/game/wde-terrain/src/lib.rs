use bevy::prelude::*;

use crate::render::TerrainRenderPlugin;

mod render;

pub mod prelude {
    pub use super::TerrainPlugin;
}

pub struct TerrainPlugin;
impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TerrainRenderPlugin);
    }
}
