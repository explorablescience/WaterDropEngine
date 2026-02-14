use bevy::prelude::*;
use wde::prelude::{Color as WdeColor, *};

pub struct TestPlugin;
impl Plugin for TestPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_scene);
    }
}

// #[derive(Resource, Default)]
// struct UiInfo(String);

// fn ui(ctx: Res<EguiContext>, info: ResMut<UiInfo>, window: Single<&Window, With<PrimaryWindow>>) {
//     // General UI parameters
//     let painter = ctx.0.debug_painter();
//     let window_size = window.size();

//     // Calculate text position
//     let text_layout = painter.layout(info.0.clone(), egui::FontId::default(), egui::Color32::WHITE, f32::INFINITY);
//     let pos = egui::Pos2::new(window_size.x - text_layout.size().x - 10.0, window_size.y - text_layout.size().y);

//     // Draw instructionss
//     painter
//         .text(pos, egui::Align2::LEFT_BOTTOM, info.0.clone(), egui::FontId::default(), egui::Color32::DARK_GRAY);
// }

fn init_scene(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Main camera
    commands.spawn((
        Transform::from_xyz(2.0, 2.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
        Camera,
        CameraView::default(),
        CameraController::default(),
        ActiveCamera,
    ));

    // Create the ground
    let scaling: u32 = 25; // Must be odd
    commands.spawn((
        Transform::from_xyz(0.0, -0.0001, 0.0).with_scale(Vec3::ONE * scaling as f32),
        Mesh(asset_server.add(PlaneMesh::from("ground", scaling, Vec3::Y))),
        GizmoMaterial(asset_server.add(GizmoMaterialAsset {
            label: "grid".to_string(),
            color: [0.8, 0.8, 0.8, 1.0],
        })),
        Collider::cuboid(50.0, 0.1, 50.0),
    ));

    // Create a typical character
    commands.spawn((
        Transform::from_xyz(0.0, 1.0, 0.0),
        Mesh(asset_server.add(CapsuleMesh::from("character", CapsuleMeshConfig::default()))),
        GizmoMaterial(asset_server.add(GizmoMaterialAsset {
            label: "character".to_string(),
            color: [0.8, 0.4, 0.4, 1.0]s
        })),
    ));

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
