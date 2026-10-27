use wde::prelude::*;

use bevy::prelude::*;

use crate::colonist::MSpawnColonist;

const DOME_FLOOR: f32 = 0.17;
const DEMO_COLONISTS: &[[f32; 3]] = &[
    [3.0, DOME_FLOOR, 3.0],
    [-1.0, DOME_FLOOR, 3.0],
    [-1.0, DOME_FLOOR, -3.0],
    [1.0, DOME_FLOOR, 3.0],
    [19.0, DOME_FLOOR, -1.0],
    [21.0, DOME_FLOOR, 1.0],
    [-9.0, 0.0, 12.5],
    [-8.0, 0.0, 13.5],
];

pub fn spawn_demo_colonists(mut commands: Commands) {
    for (i, position) in DEMO_COLONISTS.iter().enumerate() {
        commands.write_message(MSpawnColonist {
            name: format!("Colonist {}", i),
            position: Vec3::from(*position),
        });
    }
    info!("Spawned {} demo colonists", DEMO_COLONISTS.len());
}
