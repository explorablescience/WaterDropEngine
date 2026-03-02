use bevy::prelude::*;

use crate::physics::heightfield::HeightfieldPlugin;

mod heightfield;

pub struct TerrainPhysicsPlugin;
impl Plugin for TerrainPhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(HeightfieldPlugin);
    }
}
