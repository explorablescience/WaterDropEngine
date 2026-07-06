use bevy::prelude::*;
use wde::prelude::{Color as WdeColor, *};

pub struct TestPlugin;
impl Plugin for TestPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_scene).add_systems(
            Update,
            (
                add_human_collider,
                select_entity,
                set_entities_target,
                draw_gizmo_smoke_test,
            ),
        );
    }
}

#[derive(Component)]
struct EntityMarker;

/// Tracks the human entity's glTF asset handle until it has finished loading,
/// so its collider can be sized from the loaded bounding box.
#[derive(Component)]
struct PendingHumanCollider(Handle<GltfAsset>);

/// The outline material used to draw the contour line around selected entities.
#[derive(Resource)]
struct SelectionOutlineMaterial(Handle<OutlineMaterial>);

/// The mesh and translucent material used to draw the RTS-style drag-selection ground decal.
#[derive(Resource)]
struct SelectionAreaAssets {
    mesh: Handle<Mesh>,
    material: Handle<SelectionAreaMaterial>,
}

/// Marker component for the drag-selection ground decal entity.
#[derive(Component)]
struct SelectionAreaQuadMarker;

/// Vertical offset applied to the selection-area decal above the terrain hit point, to avoid z-fighting.
const SELECTION_AREA_Y_OFFSET: f32 = 0.01;

fn init_scene(
    mut commands: Commands,
    mut gltf_spawn_queue: ResMut<GltfSpawnQueue>,
    asset_server: Res<AssetServer>,
) {
    // Main camera
    commands.spawn((
        Name::new("Main Camera"),
        Transform::from_xyz(2.0, 2.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
        ActiveCamera,
        ThirdPersonController::default(), // FreeCameraController::default()
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
        ChildOf(entity),
    ));
    commands.spawn((
        Name::new("Green Light"),
        PointLight {
            position: Vec3::new(-5.0, 10.0, 5.0),
            color: WdeColor::from_srgba(0.2, 0.8, 0.2, 1.0),
            ..Default::default()
        },
        ChildOf(entity),
    ));
    commands.spawn((
        Name::new("Blue Light"),
        PointLight {
            position: Vec3::new(0.0, 8.0, -5.0),
            color: WdeColor::from_srgba(0.2, 0.2, 0.8, 1.0),
            ..Default::default()
        },
        ChildOf(entity),
    ));
    // commands.spawn((
    //     Name::new("Directional Light"),
    //     DirectionalLight {
    //         direction: Vec3::new(-1.0, -2.0, -1.0).normalize(),
    //         intensity: 0.1,
    //         ..Default::default()
    //     },
    //     ChildOf(entity)
    // ));

    commands.insert_resource(SelectionOutlineMaterial(asset_server.add(
        OutlineMaterial {
            label: "selection-outline".to_string(),
            color: WdeColor::from_srgba(1.0, 0.85, 0.0, 1.0),
            ..Default::default()
        },
    )));

    commands.insert_resource(SelectionAreaAssets {
        mesh: asset_server.add(PlaneMesh::from("selection-area-quad", 1, Vec3::Y, true)),
        material: asset_server.add(SelectionAreaMaterial {
            label: "selection-area".to_string(),
            color: WdeColor::from_srgba(0.8, 0.8, 0.4, 0.2),
            ..Default::default()
        }),
    });

    let global_parent = commands
        .spawn((Name::new("Global Parent"), Transform::default()))
        .id();
    let gltf_asset: Handle<GltfAsset> = asset_server.load("models/entities/human/human.gltf");
    for i in 0..10 {
        let parent = commands
            .spawn((
                Name::new(format!("Human {}", i)),
                Transform::from_scale(Vec3::splat(2.0)).with_translation(Vec3::new(
                    i as f32 * 1.0,
                    0.0,
                    0.0,
                )),
                PendingHumanCollider(gltf_asset.clone()),
                EntityMarker,
                ChildOf(global_parent),
            ))
            .id();
        GltfLoader::spawn(&mut gltf_spawn_queue, gltf_asset.clone(), parent);
    }
}

/// Adds a box collider to the human entity once its glTF asset has finished loading,
/// sized and offset from the model's bounding box.
fn add_human_collider(
    mut commands: Commands,
    gltf_assets: Res<Assets<GltfAsset>>,
    query: Query<(Entity, &PendingHumanCollider)>,
) {
    for (entity, pending) in &query {
        let Some(gltf_asset) = gltf_assets.get(&pending.0) else {
            continue;
        };

        let bbox = &gltf_asset.bbox;
        let bbox_extent = bbox.max - bbox.min;
        let bbox_offset = bbox.min + bbox_extent / 2.0;

        commands.entity(entity).remove::<PendingHumanCollider>();
        commands.spawn((
            Name::new("Human Collider"),
            Transform::from_translation(bbox_offset),
            Collider::from(BoxCollider::new(bbox_extent)),
            ChildOf(entity),
        ));
    }
}

#[derive(Component)]
struct EntitySelectedMarker;

#[allow(clippy::too_many_arguments)]
fn select_entity(
    mut commands: Commands,
    mut is_selecting_area: Local<Option<Vec3>>,
    mut selection_area_quad: Local<Option<Entity>>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    cursor_pos: Res<TerrainCursorPos>,
    outline_material: Res<SelectionOutlineMaterial>,
    selection_area_assets: Res<SelectionAreaAssets>,
    selected_entities: Query<Entity, With<EntitySelectedMarker>>,
    entities_query: Query<
        (Entity, &Transform),
        (With<EntityMarker>, Without<SelectionAreaQuadMarker>),
    >,
    children_query: Query<&Children>,
    mesh_query: Query<(), With<Mesh3d>>,
    mut quad_transform_query: Query<&mut Transform, With<SelectionAreaQuadMarker>>,
) {
    if mouse_input.just_pressed(MouseButton::Left) {
        let start_pos = cursor_pos.pos_or_last();
        *is_selecting_area = Some(start_pos);

        // Spawn the ground selection decal, starting as a zero-size quad at the drag origin
        let entity = commands
            .spawn((
                Name::new("Selection Area Decal"),
                SelectionAreaQuadMarker,
                Mesh3d(selection_area_assets.mesh.clone()),
                PbrMaterial3d(selection_area_assets.material.clone()),
                Transform::from_translation(
                    start_pos + Vec3::new(0.0, SELECTION_AREA_Y_OFFSET, 0.0),
                )
                .with_scale(Vec3::new(0.0, 1.0, 0.0)),
                PbrSsboTransformUuid::default(),
            ))
            .id();
        *selection_area_quad = Some(entity);
    } else if mouse_input.just_released(MouseButton::Left) {
        if let Some(start_pos) = *is_selecting_area {
            let end_pos = cursor_pos.pos_or_last();
            let min = start_pos.min(end_pos);
            let max = start_pos.max(end_pos);
            let selection_rect = Rect::new(min.x, min.z, max.x, max.z);

            // Deselect all previously selected entities and remove their contour outline
            for entity in &selected_entities {
                commands.entity(entity).remove::<EntitySelectedMarker>();
                set_entity_outline(&mut commands, &children_query, &mesh_query, entity, None);
            }

            // Select entities within the selection rectangle and add their contour outline
            for (entity, transform) in entities_query.iter() {
                let entity_pos = transform.translation;
                if selection_rect.contains(Vec2::new(entity_pos.x, entity_pos.z)) {
                    commands.entity(entity).insert(EntitySelectedMarker);
                    set_entity_outline(
                        &mut commands,
                        &children_query,
                        &mesh_query,
                        entity,
                        Some(&outline_material.0),
                    );
                }
            }
        }
        *is_selecting_area = None;

        // Despawn the selection decal now that the drag has ended
        if let Some(entity) = selection_area_quad.take() {
            commands.entity(entity).despawn();
        }
    } else if let Some(start_pos) = *is_selecting_area {
        // Still dragging: update the decal's transform to match the current drag rectangle
        if let Some(entity) = *selection_area_quad
            && let Ok(mut transform) = quad_transform_query.get_mut(entity)
        {
            let end_pos = cursor_pos.pos_or_last();
            let min = start_pos.min(end_pos);
            let max = start_pos.max(end_pos);
            let center = (min + max) * 0.5;

            transform.translation = Vec3::new(center.x, SELECTION_AREA_Y_OFFSET, center.z);
            transform.scale = Vec3::new(
                (max.x - min.x).max(f32::EPSILON),
                1.0,
                (max.z - min.z).max(f32::EPSILON),
            );
        }
    }
}

/// Adds or removes the outline material on the mesh children of `entity`, which draws (or clears)
/// the contour line effect around it. Passing `None` removes the outline.
fn set_entity_outline(
    commands: &mut Commands,
    children_query: &Query<&Children>,
    mesh_query: &Query<(), With<Mesh3d>>,
    entity: Entity,
    outline_material: Option<&Handle<OutlineMaterial>>,
) {
    let Ok(children) = children_query.get(entity) else {
        return;
    };

    for &child in children {
        if !mesh_query.contains(child) {
            continue;
        }

        match outline_material {
            Some(material) => {
                commands
                    .entity(child)
                    .insert(PbrMaterial3d(material.clone()));
            }
            None => {
                commands
                    .entity(child)
                    .remove::<PbrMaterial3d<OutlineMaterial>>();
            }
        }
    }
}

fn set_entities_target(
    selected_entities: Query<Entity, With<EntitySelectedMarker>>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    cursor_pos: Res<TerrainCursorPos>,
    mut navigator: ResMut<TerrainNavigator>,
) {
    if selected_entities.is_empty() || !mouse_input.just_pressed(MouseButton::Right) {
        return;
    }
    for entity in &selected_entities {
        let target_location = cursor_pos.pos_or_last();
        navigator.add(entity, target_location);
    }
}

// TEMP: smoke test for wde-gizmos rendering, to be removed.
fn draw_gizmo_smoke_test(mut gizmos: ResMut<Gizmos>) {
    gizmos.cube(Transform::from_xyz(0.0, 1.5, 0.0).with_scale(Vec3::splat(2.0)), WdeColor::from_srgba(1.0, 0.0, 0.0, 1.0));
    gizmos.line(Vec3::new(-5.0, 0.1, 0.0), Vec3::new(5.0, 0.1, 0.0), WdeColor::from_srgba(0.0, 1.0, 0.0, 1.0));
    gizmos.line(Vec3::new(0.0, 0.1, -5.0), Vec3::new(0.0, 0.1, 5.0), WdeColor::from_srgba(0.0, 0.5, 1.0, 1.0));
}
