use bevy::prelude::*;
use wde::prelude::{Color as WdeColor, *};

pub struct TestPlugin;
impl Plugin for TestPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_scene);
    }
}

fn init_scene(
    mut commands: Commands,
    // asset_server: Res<AssetServer>,
    // mut gltf_spawn_queue: ResMut<GltfSpawnQueue>
) {
    // Main camera
    commands.spawn((
        Name::new("Main Camera"),
        Transform::from_xyz(2.0, 2.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
        ActiveCamera,
        ThirdPersonController::default()
    ));

    // Spawn the lights
    let entity = commands.spawn(Name::new("Lights")).id();
    commands.spawn((
        Name::new("Red Light"),
        PointLight {
            position: Vec3::new(5.0, 15.0, 5.0),
            color: WdeColor::from_srgba(0.8, 0.2, 0.2, 1.0),
            ..Default::default()
        },
        ChildOf(entity)
    ));
    commands.spawn((
        Name::new("Green Light"),
        PointLight {
            position: Vec3::new(-5.0, 10.0, 5.0),
            color: WdeColor::from_srgba(0.2, 0.8, 0.2, 1.0),
            ..Default::default()
        },
        ChildOf(entity)
    ));
    commands.spawn((
        Name::new("Blue Light"),
        PointLight {
            position: Vec3::new(0.0, 8.0, -5.0),
            color: WdeColor::from_srgba(0.2, 0.2, 0.8, 1.0),
            ..Default::default()
        },
        ChildOf(entity)
    ));
    commands.spawn((
        Name::new("Directional Light"),
        DirectionalLight {
            direction: Vec3::new(-1.0, -2.0, -1.0).normalize(),
            intensity: 0.1,
            ..Default::default()
        },
        ChildOf(entity)
    ));

    // // Spawn a default gltf material
    // GltfLoader::spawn(
    //     &mut gltf_spawn_queue,
    //     asset_server.load("models/placement/house_demo1/house_demo1.gltf")
    // );
}
