use bevy::prelude::*;
use wde_render::{assets::{Mesh, materials::{PbrMaterial, PbrMaterialAsset}}, components::{ActiveCamera, Camera, CameraController, CameraView, DirectionalLight}};

pub struct ScenePlugin;
impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init);
    }
}


fn init(mut commands: Commands, asset_server: Res<AssetServer>, mut materials: ResMut<Assets<PbrMaterialAsset>>) {
    // Main camera
    commands.spawn((
        (
            Camera,
            Transform::from_xyz(5.0, 5.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
            CameraView {
                zfar: 10000.0,
                ..Default::default()
            }
        ),
        CameraController::default(),
        ActiveCamera
    ));

    let blue = materials.add(PbrMaterialAsset {
        label: "blue".to_string(),
        albedo: (0.0, 0.0, 1.0, 1.0),
        specular: 0.5,
        ..Default::default()
    });

    // Load the models
    let cube = asset_server.load("models/container.obj");
    
    // Spawn the entities
    commands.spawn((
        Transform::from_xyz(0.0, 0.0, 0.0),
        Mesh(cube.clone()),
        PbrMaterial(blue.clone())
    ));

    // Spawn the lights
    commands.spawn(DirectionalLight {
        direction: Vec3::new(-0.1, -0.8, -0.2),
        ..Default::default()
    });
}

