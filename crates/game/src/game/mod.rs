use bevy::prelude::*;
// use physics::PhysicsPlugin;
use scene::ScenePlugin;
use terrain::TerrainPlugin;

pub mod scene;
pub mod terrain;
pub mod physics;

pub struct GamePlugin;
impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        // Add the scene plugin
        app.add_plugins(ScenePlugin);

        // Add the terrain plugin
        app.add_plugins(TerrainPlugin);

        // // Add the physics plugin
        // app.add_plugins(PhysicsPlugin);
    }
}
