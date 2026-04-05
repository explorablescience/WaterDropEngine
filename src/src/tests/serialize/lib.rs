use bevy::prelude::*;
use wde_pbr::prelude::*;
use wde_renderer::prelude::*;
use wde_camera::prelude::*;

use crate::serialize::serialize_world;

mod serializers;
mod serialize;

// List of ignored components
pub(crate) const IGNORED_COMPONENTS: &[&str] = &[
    "bevy_window::monitor::Monitor",
    "bevy_window::monitor::PrimaryMonitor",
    "bevy_window::window::Window",
    "bevy_window::window::CursorOptions",
    "bevy_window::window::PrimaryWindow",
    "bevy_window::raw_handle::RawHandleWrapperHolder",
    "bevy_winit::system::CachedWindow",
    "bevy_winit::system::WinitWindowPressedKeys",
    "bevy_winit::system::CachedCursorOptions",
    "bevy_window::window::WindowScaleFactorChanged",
    "bevy_window::raw_handle::RawHandleWrapper",
    "bevy_transform::components::global_transform::GlobalTransform",
    "bevy_transform::components::transform::TransformTreeChanged"
];

pub struct ScenePlugin;
impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (init_demo_scene, serialize_world).chain());
    }
}

fn init_demo_scene(mut commands: Commands, asset_server: Res<AssetServer>, mut materials: ResMut<Assets<Material3dAsset>>) {
    // Main camera
    commands.spawn((
        Transform::from_xyz(5.0, 5.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
        Camera,
        CameraView {
            zfar: 10000.0,
            ..Default::default()
        },
        CameraController::default(),
        ActiveCamera
    ));

    let blue = materials.add(Material3dAsset {
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
        Material3d(blue.clone())
    ));

    // Spawn the lights
    commands.spawn(DirectionalLight {
        direction: Vec3::new(-0.1, -0.8, -0.2),
        ..Default::default()
    });
}

// fn test_load(mut commands: Commands, app_registry: Res<AppTypeRegistry>) {
//     let app_registry = app_registry.read();
//     const COMPONENT_DEMO: &str = r#"{"bevy_transform::components::transform::Transform":{"translation":[5.0,5.0,3.0],"rotation":[0.0,-0.2897829,0.0,0.9570924],"scale":[1.0,1.0,1.0]}}"#;

//     // Read the component and create a deserializer
//     let mut serialized = serde_json::Deserializer::from_str(COMPONENT_DEMO);

//     // Deserialize
//     let reflect_deserializer = ReflectDeserializer::new(&app_registry);
//     let deserialized = reflect_deserializer.deserialize(&mut serialized).unwrap();
//     log::info!("Deserialized value: {:?}", deserialized);

//     // Insert into the entity
//     commands.spawn_empty().insert_reflect(deserialized);
// }
