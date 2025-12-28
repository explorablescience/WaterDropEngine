use bevy::{ecs::{reflect::ReflectCommandExt}, log, prelude::*, reflect::serde::{ReflectDeserializer, ReflectSerializer}};
use serde::de::DeserializeSeed;
use wde_render::{assets::{Mesh, materials::{PbrMaterial, PbrMaterialAsset}}, components::{ActiveCamera, Camera, CameraController, CameraView, DirectionalLight}};

// List of ignored components
const IGNORED_COMPONENTS: &[&str] = &[
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
        // app.add_systems(Startup, serialize_and_deserialize_component);
        // app.add_systems(Startup, test_save);
        app.add_systems(Startup, (init, test_save).chain());
    }
}

fn test_load(mut commands: Commands, app_registry: Res<AppTypeRegistry>) {
    let app_registry = app_registry.read();
    const COMPONENT_DEMO: &str = r#"{"bevy_transform::components::transform::Transform":{"translation":[5.0,5.0,3.0],"rotation":[0.0,-0.2897829,0.0,0.9570924],"scale":[1.0,1.0,1.0]}}"#;

    // Read the component and create a deserializer
    let mut serialized = serde_json::Deserializer::from_str(&COMPONENT_DEMO);

    // Deserialize
    let reflect_deserializer = ReflectDeserializer::new(&app_registry);
    let deserialized = reflect_deserializer.deserialize(&mut serialized).unwrap();
    log::info!("Deserialized value: {:?}", deserialized);

    // Insert into the entity
    commands.spawn_empty().insert_reflect(deserialized);
}


fn test_save(world: &mut World) {
    // Get the type registry (store of all registered types)
    let app_registry = world.resource::<AppTypeRegistry>().clone();
    let app_registry = app_registry.read();

    // Select all entities in the world
    let mut query = QueryBuilder::<EntityRef>::new(world);
    let mut query = query.build();

    // Iterate over all entities
    let mut serialized_entities = Vec::new();
    for entity in query.iter(world) {
        let archetype = entity.archetype();
        let mut entity_components = Vec::new();

        // Iterate over all components
        for component_id in archetype.iter_components() {
            // Find name and type id of the component
            let (name, type_id) = match world.components().get_info(component_id) {
                Some(info) => match info.type_id() {
                    Some(type_id) => (info.name(), type_id),
                    None => continue,
                },
                None => continue,
            };
            log::info!("Serializing component: {}", name);

            // Skip ignored components
            if IGNORED_COMPONENTS.contains(&name.to_string().as_str()) {
                continue;
            }

            // Get the component data as Reflect
            let component = match app_registry.get(type_id) {
                Some(type_registration) => {
                    match type_registration.data::<ReflectComponent>() {
                        Some(reflect_component) => match reflect_component.reflect(entity) {
                            Some(component) => component,
                            None => {
                                log::warn!("Entity does not have component of type ID {:?}", type_id);
                                continue;
                            }
                        },
                        None => continue
                    }
                },
                None => continue
            };
            
            // Serialize the component
            let serializer = ReflectSerializer::new(component, &app_registry);
            let serialized_component = serde_json::to_string(&serializer).unwrap();
            entity_components.push(serialized_component);
            log::info!("Serialized component");
        }

        // Combine all components into a single JSON object
        if entity_components.is_empty() {
            continue;
        }
        let serialized_entity = format!("{{\"components\":[{}]}}", entity_components.join(","));
        log::info!("Serialized Entity: {}", serialized_entity);
        serialized_entities.push(serialized_entity);
    }

    // Combine all entities into a single JSON array
    let serialized_world = format!("[{}]", serialized_entities.join(","));
    log::info!("Serialized World: {}", serialized_world);



    // let test = Transform {
    //     translation: Vec3::new(5.0, 5.0, 3.0),
    //     rotation: Quat::from_euler(EulerRot::XYZ, 0.0, -0.588, 0.0),
    //     scale: Vec3::ONE,
    // };
    // log::info!("Original Transform: {:?}", test);

    // // Serialize
    // let registry = app_registry.read();
    // let serializer = ReflectSerializer::new(&test, &registry);
    // let serialized = serde_json::to_string(&serializer).unwrap();
    // log::info!("Serialized Transform: {}", serialized);
}


// fn serialize_and_deserialize_component() {
//     let original_value = MyStruct {
//         foo: 123
//     };

//     // Create a TypeRegistry and register MyStruct
//     let app_registry = AppTypeRegistry::default();
//     {
//         let mut registry = app_registry.write();
//         registry.register::<MyStruct>();
//     }

//     // Register
//     let deserialized_value = {
//         let registry = app_registry.read();
//         // Serialize
//         let serialized = ReflectSerializer::new(&original_value, &registry);
//         let serialized = serde_json::to_string(&serialized).unwrap();
//         log::info!("Serialized with ReflectSerializer: {}", serialized);

//         // Deserialize
//         let reflect_deserializer = ReflectDeserializer::new(&registry);
//         let mut serialized = serde_json::Deserializer::from_str(&serialized);
//         reflect_deserializer.deserialize(&mut serialized).unwrap()
//     };

//     // Insert into a world
//     let mut world = World::new();
//     world.insert_resource(app_registry);
    
//     let entity = world.spawn_empty().id();
//     world.entity_mut(entity).insert_reflect(deserialized_value);
// }


fn init(mut commands: Commands, asset_server: Res<AssetServer>, mut materials: ResMut<Assets<PbrMaterialAsset>>) {
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

