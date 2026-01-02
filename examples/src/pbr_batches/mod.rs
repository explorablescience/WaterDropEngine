use bevy::prelude::*;
use wde::prelude::*;

pub struct PbrBatchesPlugin;
impl Plugin for PbrBatchesPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(PbrPlugin)
            .add_systems(Startup, init);
    }
}

fn init(mut commands: Commands, asset_server: Res<AssetServer>, mut materials: ResMut<Assets<PbrMaterialAsset>>) {
    // Creates a camera
    commands.spawn(
        ((
            Camera,
            Transform::from_xyz(2.0, 2.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y)
        ),
        CameraController::default()
    ));

    // Load the assets
    let box_texture = asset_server.load_with_settings("examples/pbr_batches/box.png", |settings: &mut TextureLoaderSettings| {
        settings.label = "pbr-box".to_string();
        settings.format = TextureFormat::Rgba8Unorm;
        settings.usages = TextureUsages::TEXTURE_BINDING;
    });
    let red_box = materials.add(PbrMaterialAsset {
        label: "pbr-material-red-box".to_string(),
        albedo_t: Some(box_texture),
        ..Default::default()
    });
    let blue = materials.add(PbrMaterialAsset {
        label: "pbr-material-blue".to_string(),
        albedo: (0.0, 0.0, 1.0, 1.0),
        ..Default::default()
    });
    let suzanne = asset_server.load("examples/pbr_batches/suzanne.obj");
    let cube = asset_server.load("examples/pbr_batches/cube.obj");

    // Spawn the entities
    commands.spawn((
        Transform::from_xyz(0.0, 0.0, 0.0),
        Mesh(cube.clone()),
        PbrMaterial(blue.clone())
    ));
    commands.spawn((
        Transform::from_xyz(5.0, 0.0, 0.0),
        Mesh(cube.clone()),
        PbrMaterial(blue.clone())
    ));
    commands.spawn((
        Transform::from_xyz(10.0, 0.0, 0.0),
        Mesh(cube.clone()),
        PbrMaterial(red_box.clone())
    ));
    commands.spawn((
        Transform::from_xyz(15.0, 0.0, 0.0),
        Mesh(suzanne.clone()),
        PbrMaterial(red_box.clone())
    ));
    for i in 1..100 {
        for j in 1..100 {
            commands.spawn((
                Transform::from_xyz(i as f32 * 5.0, 0.0, j as f32 * 5.0 + 5.0),
                Mesh(suzanne.clone()),
                PbrMaterial(blue.clone())
            ));
        }
    }
}
