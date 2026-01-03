use bevy::prelude::*;
use wde_camera::prelude::*;
use wde_pbr::prelude::*;
use wde_renderer::prelude::*;
use wde_gizmos::prelude::*;

use crate::physics::{Collider, PhysicsPlugin};

mod physics;

pub struct ScenePlugin;
impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(TimerResource(Timer::from_seconds(5.0, TimerMode::Once)))
            .add_plugins(PhysicsPlugin)
            .add_systems(Startup, init_scene)
            .add_systems(Update, remove_after_time);
    }
}

#[derive(Resource)]
struct CubeEntity(Entity);
#[derive(Resource)]
struct GroundEntity(Entity);

fn init_scene(mut commands: Commands, asset_server: Res<AssetServer>, mut pbrmaterials: ResMut<Assets<PbrMaterialAsset>>, mut gizmomaterials: ResMut<Assets<GizmoMaterialAsset>>, mut meshes: ResMut<Assets<MeshAsset>>) {
    // Main camera
    commands.spawn((
        Transform::from_xyz(5.0, 5.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
        Camera,
        CameraView::default(),
        CameraController::default(),
        ActiveCamera
    ));

    // Spawn a cube centered at the origin
    let cube_entity =commands.spawn((
        Transform::from_xyz(0.0, 0.0, 0.0).with_scale(Vec3::ONE * 0.1),
        Mesh(asset_server.load("models/container.obj")),
        PbrMaterial(pbrmaterials.add(PbrMaterialAsset {
            label: "red".to_string(),
            albedo: (1.0, 0.0, 0.0, 1.0),
            specular: 0.5,
            ..Default::default()
        }))
    )).id();
    commands.insert_resource(CubeEntity(cube_entity));

    // Create the ground
    let ground = commands.spawn((
        Transform::from_xyz(0.0, -1.0, 0.0).with_scale(Vec3::ONE * 100.0),
        Mesh(meshes.add(PlaneMesh::from("ground", 100, Vec3::Y))),
        GizmoMaterial(gizmomaterials.add(GizmoMaterialAsset {
            label: "grid".to_string(),
            color: [0.8, 0.8, 0.8, 1.0],
        })),
        Collider::cuboid(50.0, 0.1, 50.0),
    )).id();
    commands.insert_resource(GroundEntity(ground));

    // Spawn the lights
    commands.spawn(DirectionalLight {
        direction: Vec3::new(-0.1, -1.2, -0.2),
        ..Default::default()
    });
}

#[derive(Resource)]
struct TimerResource(Timer);

fn remove_after_time(
    mut commands: Commands,
    time: Res<Time>,
    ground_entity: Res<GroundEntity>,
    mut timer: ResMut<TimerResource>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        info!("Removing cube after 5 seconds");
        commands.entity(ground_entity.0).despawn();
    }
}
