use bevy::prelude::*;
use wde::prelude::{Color as WdeColor, *};

pub struct TestPlugin;
impl Plugin for TestPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(TerrainTerraformPlugin)
            .add_systems(Startup, init_scene)
            .add_systems(Update, edit_camera_params);
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
        ThirdPersonController::default(),
        ActiveCamera,
    ));

    // // Create the ground
    // let scaling: u32 = 25; // Must be odd
    // commands.spawn((
    //     Transform::from_xyz(0.0, -0.0001, 0.0).with_scale(Vec3::ONE * scaling as f32),
    //     Mesh(asset_server.add(PlaneMesh::from("ground", scaling, Vec3::Y))),
    //     GizmoMaterial(asset_server.add(GizmoMaterialAsset {
    //         label: "grid".to_string(),
    //         color: [0.8, 0.8, 0.8, 1.0],
    //     })),
    //     Collider::cuboid(50.0, 0.1, 50.0),
    // ));

    // Create a typical character
    // commands.spawn((
    //     Transform::from_xyz(0.0, 1.0, 0.0),
    //     Mesh(asset_server.add(CapsuleMesh::from("character", CapsuleMeshConfig::default()))),
    //     GizmoMaterial(asset_server.add(GizmoMaterialAsset {
    //         label: "character".to_string(),
    //         color: [0.8, 0.4, 0.4, 1.0]
    //     })),
    // ));

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

fn edit_camera_params(ctx: Res<EguiContext>, mut camera_query: Query<(&mut CameraView, &mut ThirdPersonController), With<ActiveCamera>>) {
    if let Ok((mut view, mut controller)) = camera_query.single_mut() {
        // egui::Window::new("Terrain Painting").show(&ctx.0, |ui| {
        //     ui.add(egui::Slider::new(&mut controller.sensitivity, 0.001..=1.0).text("Sensitivity"));
        //     ui.add(egui::Slider::new(&mut controller.friction, 0.0..=1.0).text("Friction"));
        //     ui.add(egui::Slider::new(&mut controller.min_move_speed, 0.1..=5000.0).text("Min Move Speed"));
        //     ui.add(egui::Slider::new(&mut controller.max_move_speed, 0.1..=5000.0).text("Max Move Speed"));
        //     ui.add(egui::Slider::new(&mut controller.run_speed_multiplier, 1.0..=10.0).text("Run Speed Multiplier"));
        //     ui.add(egui::Slider::new(&mut controller.zoom_speed, 0.1..=100.0).text("Zoom Speed"));
        //     ui.add(egui::Slider::new(&mut controller.distance, 1.0..=1000.0).text("Distance"));
        //     ui.add(egui::Slider::new(&mut controller.min_distance, 0.1..=1000.0).text("Min Distance"));
        //     ui.add(egui::Slider::new(&mut controller.max_distance, 1.0..=1000.0).text("Max Distance"));
        //     ui.add(egui::Slider::new(&mut controller.min_pitch, 0.0..=std::f32::consts::FRAC_PI_2).text("Min Pitch"));
        //     ui.add(egui::Slider::new(&mut controller.max_pitch, 0.0..=std::f32::consts::FRAC_PI_2).text("Max Pitch"));
        //     ui.add(egui::Slider::new(&mut controller.edge_scroll_distance, 0.0..=200.0).text("Edge Scroll Distance"));
        //     ui.add(egui::Slider::new(&mut controller.edge_scroll_speed, 0.0..=1000.0).text("Edge Scroll Speed"));

        //     // Reset button
        //     if ui.button("Reset").clicked() {
        //         *controller = ThirdPersonController::default();
        //     }
        // });
    }
}
