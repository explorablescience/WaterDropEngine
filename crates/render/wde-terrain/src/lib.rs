use bevy::prelude::*;

pub mod prelude {
    pub use super::TerrainPlugin;
}

pub struct TerrainPlugin;
impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        // Register terrain systems and resources here
    }
}
