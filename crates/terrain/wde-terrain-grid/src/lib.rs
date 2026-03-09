use bevy::prelude::*;

pub mod prelude {
    pub use super::TerrainGridPlugin;
}

pub struct TerrainGridPlugin;
impl Plugin for TerrainGridPlugin {
    fn build(&self, app: &mut App) {
        // Register systems and resources here
    }
}
