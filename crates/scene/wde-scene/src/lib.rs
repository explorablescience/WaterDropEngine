use wde_gltf::prelude::*;
use bevy::prelude::*;
use wde_camera::prelude::*;
use wde_pbr::prelude::*;
use wde_renderer::prelude::*;
use wde_gizmos::prelude::*;

pub struct ScenePlugin;
impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app
            // .insert_resource(TimerResource(Timer::from_seconds(5.0, TimerMode::Once)))
            .add_systems(Startup, (init_scene, load).chain());
    }
}

// #[derive(Resource)]
// struct GroundEntity(Entity);

fn init_scene(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Main camera
    commands.spawn((
        Transform::from_xyz(2.0, 2.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
        Camera,
        CameraView::default(),
        CameraController::default(),
        ActiveCamera
    ));

    // Create the ground
    let scaling: u32 = 25; // Must be odd
    let _ground = commands.spawn((
        Transform::from_xyz(0.0, -0.0001, 0.0).with_scale(Vec3::ONE * scaling as f32),
        Mesh(asset_server.add(PlaneMesh::from("ground", scaling, Vec3::Y))),
        GizmoMaterial(asset_server.add(GizmoMaterialAsset {
            label: "grid".to_string(),
            color: [0.8, 0.8, 0.8, 1.0],
        })),
        // Collider::cuboid(50.0, 0.1, 50.0),
    )).id();
    // commands.insert_resource(GroundEntity(ground));

    // Spawn the lights
    commands.spawn(DirectionalLight {
        direction: Vec3::new(-0.1, -1.2, -0.2),
        ..Default::default()
    });
    commands.spawn(DirectionalLight {
        direction: Vec3::new(0.1, 1.2, 0.2),
        ..Default::default()
    });
}


fn load(mut commands: Commands, asset_server: Res<AssetServer>) {
    // GltfLoader::load("models/FlightHelmet/FlightHelmet.gltf", &asset_server)
    //     .map(|asset| asset.spawn(&mut commands, Transform::from_scale(Vec3::ONE * 5.0)))
    //     .unwrap();

    GltfLoader::load("houses/house_demo1.gltf", &asset_server)
        .map(|asset| asset.spawn(&mut commands, Transform::from_xyz(2.0, 0.0, 0.0).with_scale(Vec3::ONE * 0.3)))
        .unwrap();

    // GltfLoader::load("models/chess_game/ABeautifulGame.gltf", &asset_server)
    //     .map(|asset| asset.spawn(&mut commands, Transform::from_scale(Vec3::ONE * 10.0)))
    //     .unwrap();
}

// fn cast_ray(
//     mut commands: Commands,
//     phworld: Res<PhysicsWorld>,
//     window: Single<&Window, With<PrimaryWindow>>,
//     cube_entity_res: Res<CubeEntity>,
//     camera_query: Query<(&Transform, &CameraView), With<Camera>>)
// {
//     // Create the ray from ndc position
//     let cursor_ndc_position = match window.cursor_position() {
//         Some(pos) => pos / window.size(),
//         None => return,
//     };
//     let (camera_transform, camera_view) = camera_query.single().map_err(|_| "No camera found").unwrap();
//     let ray = Ray::from_ndc(cursor_ndc_position, window.size().x / window.size().y, camera_transform, camera_view);

//     // Cast the ray in the physics world
//     if let Some((_, toi)) = phworld.as_ref().cast_ray(&ray, &RayCastConfig::default()) {
//         let hit_point = ray.point_at(toi);

//         // Move the cube up to the position of the hit point.
//         commands.entity(cube_entity_res.0).insert(Transform::from_translation(hit_point));
//     }
// }

// #[derive(Resource)]
// struct TimerResource(Timer);

// fn remove_after_time(
//     mut commands: Commands,
//     time: Res<Time>,
//     ground_entity: Res<GroundEntity>,
//     mut timer: ResMut<TimerResource>,
// ) {
//     if timer.0.tick(time.delta()).just_finished() {
//         info!("Removing cube after 5 seconds");
//         commands.entity(ground_entity.0).despawn();
//     }
// }
