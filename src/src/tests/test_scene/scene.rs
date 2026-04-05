use wde::{assets::{Mesh, materials::{Material3d, Material3dAsset}, meshes::PlaneMesh}, bevy::prelude::*, components::{ActiveCamera, Camera, CameraController, CameraView, DirectionalLight, PointLight, SpotLight}};
// use super::terrain::TerrainSpawner;

pub struct ScenePlugin;
impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init);
    }
}

fn init(mut commands: Commands, asset_server: Res<AssetServer>, mut materials: ResMut<Assets<Material3dAsset>>) {
    // Main camera
    commands.spawn((
        (
            Camera,
            Transform::from_xyz(5.0, 20.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
            CameraView {
                zfar: 10000.0,
                ..Default::default()
            }
        ),
        CameraController::default(),
        ActiveCamera,
        // TerrainSpawner::default()
    ));

    // Dummy pbr object
    commands.spawn((
        Transform::IDENTITY.with_scale(Vec3::splat(0.0)),
        Mesh(asset_server.add(PlaneMesh::from("dummy", [1.0, 1.0]))),
        Material3d(materials.add(Material3dAsset {
            label: "dummy".to_string(),
            ..Default::default()
        }))
    ));

    // Load the materials
    // let container_albedo = asset_server.load_with_settings("models/container_albedo.png", |settings: &mut TextureLoaderSettings| {
    //     settings.label = "container-albedo".to_string();
    //     settings.format = WTextureFormat::Rgba8UnormSrgb;
    //     settings.usages = WTextureUsages::TEXTURE_BINDING;
    // });
    // let container_specular = asset_server.load_with_settings("models/container_specular.png", |settings: &mut TextureLoaderSettings| {
    //     settings.label = "container-specular".to_string();
    //     settings.format = WTextureFormat::R8Unorm;
    //     settings.usages = WTextureUsages::TEXTURE_BINDING;
    // });
    // let red_box = materials.add(Material3dAsset {
    //     label: "container".to_string(),
    //     albedo: (1.0, 0.0, 0.0, 1.0),
    //     // albedo_t:   Some(container_albedo),
    //     // specular_t: Some(container_specular),
    //     ..Default::default()
    // });
    // let blue = materials.add(Material3d {
    //     label: "blue".to_string(),
    //     albedo: (0.0, 0.0, 1.0, 1.0),
    //     specular: 0.5,
    //     ..Default::default()
    // });

    // Load the models
    // let suzanne = asset_server.load("models/suzanne.obj");
    // let cube = asset_server.load("models/container.obj");

    // Spawn the entities
    // commands.spawn((
    //     Transform::from_xyz(0.0, 0.0, 0.0),
    //     Mesh(cube.clone()),
    //     Material3d(red_box.clone())
    // ));
    // commands.spawn(PbrBundle {
    //     transform: Transform::from_xyz(5.0, 0.0, 0.0).with_scale(Vec3::splat(3.0)),
    //     mesh: cube.clone(),
    //     material: red_box.clone()
    // });
    // commands.spawn(PbrBundle {
    //     transform: Transform::from_xyz(10.0, 0.0, 0.0),
    //     mesh: cube.clone(),
    //     material: blue.clone()
    // });
    // commands.spawn(PbrBundle {
    //     transform: Transform::from_xyz(15.0, 0.0, 0.0),
    //     mesh: suzanne.clone(),
    //     material: blue.clone()
    // });
    // commands.spawn(PbrBundle {
    //     transform: Transform::from_xyz(20.0,0.0, 0.0),
    //     mesh: cube.clone(),
    //     material: blue.clone()
    // });
    // commands.spawn(PbrBundle {
    //     transform: Transform::from_xyz(25.0, 0.0, 0.0),
    //     mesh: cube.clone(),
    //     material: red_box.clone()
    // });
    // for i in 1..100 {
    //     for j in 1..100 {
    //         let angle = i as f32 * 0.05;
    //         let axis = if i % 2 == 0 { Vec3::Y } else if i % 3 == 0 { Vec3::Z } else { Vec3::X };
    //         commands.spawn(PbrBundle {
    //             transform: Transform::from_xyz(i as f32 * 5.0, 0.0, j as f32 * 5.0 + 5.0)
    //                 .with_rotation(Quat::from_axis_angle(axis, angle)),
    //             mesh: cube.clone(),
    //             material: red_box.clone()
    //         });
    //     }
    // }
    
    // // Load the materials
    // let planks_albedo = asset_server.load_with_settings("models/planks_albedo.jpg", |settings: &mut TextureLoaderSettings| {
    //     settings.label = "planks-albedo".to_string();
    //     settings.format = WTextureFormat::Rgba8UnormSrgb;
    //     settings.usages = WTextureUsages::TEXTURE_BINDING;
    // });
    // let planks = materials.add(Material3d {
    //     label: "planks".to_string(),
    //     albedo_t: Some(planks_albedo),
    //     specular: 0.8,
    //     ..Default::default()
    // });

    // // Spawn the ground
    // let ground = asset_server.add(PlaneMesh::from("ground", [200.0, 200.0]));
    // commands.spawn(PbrBundle {
    //     transform: Transform::from_xyz(0.0, 0.0, 0.0),
    //     mesh: ground,
    //     material: planks
    // });

    // Spawn the lights
    commands.spawn(PointLight {
        position: Vec3::new(10.0, 3.0, 20.0),
        ..Default::default()
    }.with_range(100.0).unwrap());
    commands.spawn(SpotLight {
        position: Vec3::new(70.0, 5.0, 40.0),
        direction: Vec3::new(-0.8, -0.5, 0.0),
        ..Default::default()
    });
    commands.spawn(SpotLight {
        position: Vec3::new(120.0, 20.0, 156.0),
        direction: Vec3::new(0.8, -0.5, -0.1),
        ..Default::default()
    });
    commands.spawn(DirectionalLight {
        direction: Vec3::new(-0.1, -0.8, -0.2),
        ..Default::default()
    });
}
