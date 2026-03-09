use bevy::prelude::*;
use wde::prelude::{Color as WdeColor, *};

pub struct TestPlugin;
impl Plugin for TestPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, init_scene);
    }
}

fn init_scene(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Main camera
    commands.spawn((
        Transform::from_xyz(2.0, 2.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
        Camera,
        CameraView::default(),
        ThirdPersonController::default(),
        ActiveCamera,
    ));

    // Load a glTF model and spawn it in the scene
    let gltf_asset = GltfLoader::load("tests/models/houses/house_demo1.gltf", &asset_server).unwrap();
    let model = PbrModel(gltf_asset.models.clone());
    commands.spawn((model.clone(), Transform::default().with_translation(Vec3::new(0.0, 0.01, 0.0)).with_scale(Vec3::ONE * 0.8)));

    // Spawn the lights
    commands.spawn(PointLight {
        position: Vec3::new(5.0, 15.0, 5.0),
        color: WdeColor::from_srgba(0.8, 0.2, 0.2, 1.0),
        ..Default::default()
    });
    commands.spawn(PointLight {
        position: Vec3::new(-5.0, 10.0, 5.0),
        color: WdeColor::from_srgba(0.2, 0.8, 0.2, 1.0),
        ..Default::default()
    });
    commands.spawn(PointLight {
        position: Vec3::new(0.0, 8.0, -5.0),
        color: WdeColor::from_srgba(0.2, 0.2, 0.8, 1.0),
        ..Default::default()
    });
    commands.spawn(DirectionalLight {
        direction: Vec3::new(-1.0, -2.0, -1.0).normalize(),
        intensity: 0.1,
        ..Default::default()
    });
}
