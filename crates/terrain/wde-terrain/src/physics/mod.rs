use bevy::prelude::*;

use crate::physics::terrain_physics::TerrainPhysics;

pub mod terrain_physics;

pub struct TerrainPhysicsPlugin;
impl Plugin for TerrainPhysicsPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Update, TerrainPhysics::extract_dirty);
    }
}
