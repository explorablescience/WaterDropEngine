use bevy::prelude::*;
use wde::prelude::{Color as WdeColor, *};

pub struct TestPlugin;
impl Plugin for TestPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_scene);
    }
}

fn init_scene(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Main camera
    commands.spawn((
        Name::new("Main Camera"),
        Transform::from_xyz(2.0, 2.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
        Camera,
        CameraView::default(),
        ThirdPersonController::default(),
        ActiveCamera
    ));

    // Spawn the lights
    commands.spawn((
        Name::new("Red Light"),
        PointLight {
            position: Vec3::new(5.0, 15.0, 5.0),
            color: WdeColor::from_srgba(0.8, 0.2, 0.2, 1.0),
            ..Default::default()
        }
    ));
    commands.spawn((
        Name::new("Green Light"),
        PointLight {
            position: Vec3::new(-5.0, 10.0, 5.0),
            color: WdeColor::from_srgba(0.2, 0.8, 0.2, 1.0),
            ..Default::default()
        }
    ));
    commands.spawn((
        Name::new("Blue Light"),
        PointLight {
            position: Vec3::new(0.0, 8.0, -5.0),
            color: WdeColor::from_srgba(0.2, 0.2, 0.8, 1.0),
            ..Default::default()
        }
    ));
    commands.spawn((
        Name::new("Directional Light"),
        DirectionalLight {
            direction: Vec3::new(-1.0, -2.0, -1.0).normalize(),
            intensity: 0.1,
            ..Default::default()
        }
    ));

    // Spawn a default gltf material
    let model = GltfLoader::load(
        "models/placement/house_demo1/house_demo1.gltf",
        &asset_server
    )
    .unwrap();
    commands.spawn((
        Transform::from_translation(Vec3::ZERO).with_scale(Vec3::splat(1.0)),
        PbrModel(model.models)
    ));
}
