use bevy::prelude::*;
use wde::prelude::*;

const DEMO_BASE: &[(&str, [f32; 2], GridRotation)] = &[
    // Habitat
    ("Dome Medium", [1.0, 1.0], GridRotation::R0),
    ("Control Room", [1.0, -9.0], GridRotation::R0),
    ("Tunnel Straight", [9.0, 1.0], GridRotation::R0),
    ("Tunnel Straight", [11.0, 1.0], GridRotation::R0),
    ("Tunnel Straight", [13.0, 1.0], GridRotation::R0),
    ("Dome Small", [19.0, 1.0], GridRotation::R0),
    ("Tunnel Straight", [1.0, 9.0], GridRotation::R90),
    ("Tunnel Straight", [1.0, 11.0], GridRotation::R90),
    ("Dome Small", [1.0, 17.0], GridRotation::R0),
    // Way out
    ("Tunnel Straight", [-7.0, 1.0], GridRotation::R0),
    ("Airlock", [-9.0, 1.0], GridRotation::R0),
    ("Path Straight", [-11.0, 1.0], GridRotation::R0),
    // Path spine along x = -13
    ("Path Intersection", [-13.0, -9.0], GridRotation::R0),
    ("Path Intersection", [-13.0, -7.0], GridRotation::R0),
    ("Path Intersection", [-13.0, -5.0], GridRotation::R0),
    ("Path Intersection", [-13.0, -3.0], GridRotation::R0),
    ("Path Intersection", [-13.0, -1.0], GridRotation::R0),
    ("Path Intersection", [-13.0, 1.0], GridRotation::R0),
    ("Path Intersection", [-13.0, 3.0], GridRotation::R0),
    ("Path Intersection", [-13.0, 5.0], GridRotation::R0),
    ("Path Intersection", [-13.0, 7.0], GridRotation::R0),
    ("Path Intersection", [-13.0, 9.0], GridRotation::R0),
    ("Path Intersection", [-13.0, 11.0], GridRotation::R0),
    // Along the spine, toward the base
    ("Solar Array", [-10.0, 4.0], GridRotation::R0),
    ("Solar Array", [-10.0, 8.0], GridRotation::R0),
    ("Sensor Tower", [-11.0, 11.0], GridRotation::R0),
    ("Battery", [-11.0, -1.0], GridRotation::R0),
    ("Oxygen Generator", [-10.0, -4.0], GridRotation::R0),
    ("Fuel Refinery", [-10.0, -8.0], GridRotation::R0),
    // Along the spine, away from the base
    ("Concrete Extractor", [-16.0, 10.0], GridRotation::R0),
    ("Oxygen Tank", [-15.0, 7.0], GridRotation::R0),
    ("Water Extractor", [-16.0, 4.0], GridRotation::R0),
    ("Water Tank", [-15.0, 1.0], GridRotation::R0),
    ("Storage", [-17.0, -3.0], GridRotation::R0),
    ("Landing Pad", [-17.0, -9.0], GridRotation::R0),
    // Modules in the medium dome
    ("Living Quarters", [4.0, 0.0], GridRotation::R0),
    ("Hydroponic Farm", [-2.0, 0.0], GridRotation::R0),
    ("Garden", [1.0, 1.0], GridRotation::R0),
    ("Diner", [1.0, 5.0], GridRotation::R270),
    ("Grocery", [3.0, 5.0], GridRotation::R270),
    ("Residence", [-1.0, 5.0], GridRotation::R270),
    ("Residence", [1.0, -3.0], GridRotation::R90),
    ("Infirmary", [3.0, -3.0], GridRotation::R90),
    // Modules in the north dome
    ("Research Lab", [2.0, 17.0], GridRotation::R0),
    ("Infirmary", [-1.0, 17.0], GridRotation::R180),
    ("Garden", [1.0, 19.0], GridRotation::R0),
    // Modules in the east dome
    ("Residence", [17.0, 1.0], GridRotation::R180),
    ("Residence", [19.0, 3.0], GridRotation::R270),
    ("Grocery", [21.0, -1.0], GridRotation::R0),
];

pub fn spawn_demo_base(mut placement_queue: ResMut<PlacementQueue>) {
    for (label, [x, z], rotation) in DEMO_BASE {
        placement_queue.place(*label, Vec2::new(*x, *z), *rotation);
    }
    info!("Placed {} demo base modules", DEMO_BASE.len());
}
