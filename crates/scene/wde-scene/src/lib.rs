use bevy::prelude::*;
use wde_camera::prelude::*;
use wde_pbr::prelude::*;
use wde_renderer::prelude::*;

pub struct ScenePlugin;
impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_scene);
    }
}

fn init_scene(mut commands: Commands, asset_server: Res<AssetServer>, mut materials: ResMut<Assets<PbrMaterialAsset>>) {
    // Main camera
    commands.spawn((
        Transform::from_xyz(5.0, 5.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
        Camera,
        CameraView::default(),
        CameraController::default(),
        ActiveCamera
    ));

    // Create materials
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
