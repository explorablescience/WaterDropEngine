use bevy::prelude::*;
use wde::prelude::{Color as WdeColor, *};

pub struct TestPlugin;
impl Plugin for TestPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_scene)
            .add_systems(Update, set_stencil)
            .init_resource::<SpawnEntity>();
    }
}

#[derive(Resource, Default)]
struct SpawnEntity(Option<Entity>);

fn init_scene(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut gltf_spawn_queue: ResMut<GltfSpawnQueue>
) {
    // Main camera
    commands.spawn((
        Name::new("Main Camera"),
        Transform::from_xyz(2.0, 2.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
        ActiveCamera,
        ThirdPersonController::default()
    ));

    // Spawn the lights
    let entity = commands.spawn(Name::new("Lights")).id();
    commands.spawn((
        Name::new("Red Light"),
        PointLight {
            position: Vec3::new(5.0, 15.0, 5.0),
            color: WdeColor::from_srgba(0.8, 0.2, 0.2, 1.0),
            ..Default::default()
        },
        ChildOf(entity)
    ));
    commands.spawn((
        Name::new("Green Light"),
        PointLight {
            position: Vec3::new(-5.0, 10.0, 5.0),
            color: WdeColor::from_srgba(0.2, 0.8, 0.2, 1.0),
            ..Default::default()
        },
        ChildOf(entity)
    ));
    commands.spawn((
        Name::new("Blue Light"),
        PointLight {
            position: Vec3::new(0.0, 8.0, -5.0),
            color: WdeColor::from_srgba(0.2, 0.2, 0.8, 1.0),
            ..Default::default()
        },
        ChildOf(entity)
    ));
    commands.spawn((
        Name::new("Directional Light"),
        DirectionalLight {
            direction: Vec3::new(-1.0, -2.0, -1.0).normalize(),
            intensity: 0.1,
            ..Default::default()
        },
        ChildOf(entity)
    ));

    // Spawn a default gltf material
    let parent = commands
        .spawn((Name::new("GLTF Model Parent"), Transform::default()))
        .id();
    commands.insert_resource(SpawnEntity(Some(parent)));
    GltfLoader::spawn(
        &mut gltf_spawn_queue,
        asset_server.load_with_settings(
            "models/placement/house_demo1/house_demo1.gltf",
            |settings: &mut GltfLoaderSettings| settings.stencil_value = Some(1)
        ),
        parent
    );
}

fn set_stencil(
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    spawn_entity: Res<SpawnEntity>,
    children: Query<&Children>,
    query: Query<&Mesh3d>,
    mut is_set: Local<bool>
) {
    if *is_set {
        return;
    }
    if let Some(parent) = spawn_entity.0
        && let Ok(children) = children.get(parent)
    {
        for child in children {
            let mesh = query.get(*child).unwrap();
            commands.spawn((
                Name::new("Stencil Mark"),
                ChildOf(*child),
                Transform::IDENTITY,
                Mesh3d(mesh.0.clone()),
                PbrMaterial3d(asset_server.add(OutlineMaterial {
                    color: (0.5, 0.5, 0.2, 0.3),
                    thickness: 0.02,
                    ..Default::default()
                })),
                OutlineMarker
            ));
        }
        *is_set = true;
    }
}
