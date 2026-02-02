use wde::prelude::{*, Color as WdeColor};
use bevy::{prelude::*, window::PrimaryWindow};

pub struct TestPlugin;
impl Plugin for TestPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, (init_scene, load))
            .add_systems(Update, (rotate_light_sources, ui, cast_ray))
            .init_resource::<SelectEntity>()
            .init_resource::<UiInfo>();
    }
}


#[derive(Resource, Default)]
struct SelectEntity(Option<Entity>);

#[derive(Resource, Default)]
struct UiInfo(String);

fn ui(ctx: Res<EguiContext>, info: ResMut<UiInfo>, window: Single<&Window, With<PrimaryWindow>>) {
    // General UI parameters
    let painter = ctx.0.debug_painter();
    let window_size = window.size();

    // Calculate text position
    let text_layout = painter.layout(info.0.clone(), egui::FontId::default(), egui::Color32::WHITE, f32::INFINITY);
    let pos = egui::Pos2::new(window_size.x - text_layout.size().x - 10.0, window_size.y - text_layout.size().y);
    
    // Draw instructionss
    painter
        .text(pos, egui::Align2::LEFT_BOTTOM, info.0.clone(), egui::FontId::default(), egui::Color32::DARK_GRAY);
}



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
    commands.spawn((
        Transform::from_xyz(0.0, -0.0001, 0.0).with_scale(Vec3::ONE * scaling as f32),
        Mesh(asset_server.add(PlaneMesh::from("ground", scaling, Vec3::Y))),
        GizmoMaterial(asset_server.add(GizmoMaterialAsset {
            label: "grid".to_string(),
            color: [0.8, 0.8, 0.8, 1.0],
        })),
        Collider::cuboid(50.0, 0.1, 50.0),
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

fn rotate_light_sources(mut lights: Query<&mut PointLight>, time: Res<Time>) {
    for (i, mut light) in lights.iter_mut().enumerate() {
        let angle = time.elapsed_secs() * 0.5 + (i as f32) * std::f32::consts::FRAC_PI_2;
        let radius = 5.0;
        light.position.x = radius * angle.cos();
        light.position.z = radius * angle.sin();
    }
}


fn load(mut commands: Commands, asset_server: Res<AssetServer>, mut selected_entity: ResMut<SelectEntity>) {
    // let gltf_asset = GltfLoader::load("tests/models/FlightHelmet/FlightHelmet.gltf", &asset_server).unwrap();
    let gltf_asset = GltfLoader::load("tests/models/houses/house_demo1.gltf", &asset_server).unwrap();
    let model = PbrModel(gltf_asset.models.clone());

    let gltf = commands.spawn((model.clone(), Transform::default())).id();
    selected_entity.0 = Some(gltf);

    // Spawn a grid of models
    const N: i32 = 100;
    let spacing = 4.0;
    for i in 0..N {
        for j in 0..N {
            let pos = Vec3::new(
                (i as f32 - N as f32 / 2.0) * spacing,
                0.0,
                (j as f32 - N as f32 / 2.0) * spacing,
            );
            let t = Transform::from_translation(pos).with_scale(Vec3::ONE * 0.1);
            commands.spawn((model.clone(), t));
        }
    }
}

fn cast_ray(
    mut commands: Commands,
    phworld: Res<PhysicsWorld>,
    window: Single<&Window, With<PrimaryWindow>>,
    selected_entity: Res<SelectEntity>,
    camera_query: Query<(&Transform, &CameraView), With<Camera>>,
    mut ui_info: ResMut<UiInfo>,
) {
    // Return if no entity is selected
    if selected_entity.0.is_none() {
        return;
    }

    // Create the ray from ndc position
    let cursor_ndc_position = match window.cursor_position() {
        Some(pos) => pos / window.size(),
        None => return,
    };
    let (camera_transform, camera_view) = camera_query.single().map_err(|_| "No camera found").unwrap();
    let ray = Ray::from_ndc(cursor_ndc_position, window.size().x / window.size().y, camera_transform, camera_view);

    // Cast the ray in the physics world
    if let Some((_, toi)) = phworld.as_ref().cast_ray(&ray, &RayCastConfig::default()) {
        let hit_point = ray.point_at(toi);

        // Move the cube up to the position of the hit point.
        commands.entity(selected_entity.0.unwrap()).insert(Transform::from_translation(hit_point).with_scale(Vec3::ONE * 0.1));
        ui_info.0 = format!("Hit at point: ({:.2}, {:.2}, {:.2})", hit_point.x, hit_point.y, hit_point.z);
    }
}
