use bevy::{prelude::*, window::PrimaryWindow};
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
                swap_colonist_model,
                animate_colonists,
                outline_new_meshes
            )
        );
    }
}

#[derive(Component)]
pub(crate) struct EntityMarker;

/// Tracks the human entity's glTF asset handle until it has finished loading,
/// so its collider can be sized from the loaded bounding box.
#[derive(Component)]
struct PendingHumanCollider(Handle<GltfAsset>);

/// Colonist models: with a suit outdoors, bare-headed in pressurized places.
#[derive(Resource)]
struct ColonistModels {
    suit: Handle<GltfAsset>,
    indoor: Handle<GltfAsset>
}

/// Outline materials of selected and hovered entities.
#[derive(Resource)]
struct SelectionOutlineMaterial {
    selected: Handle<OutlineMaterial>,
    hovered: Handle<OutlineMaterial>
}

/// Collision group of the colonist picking colliders.
const COLONIST_COLLIDER_GROUP: ColliderGroup = ColliderGroup::GROUP_3;
/// Colonist picking colliders are wider than the model, to be easier to click.
const COLONIST_PICK_MARGIN: f32 = 1.6;
/// Cursor travel (in pixels) beyond which a press becomes a drag selection.
const DRAG_THRESHOLD: f32 = 6.0;

/// The mesh and translucent material used to draw the RTS-style drag-selection ground decal.
#[derive(Resource)]
struct SelectionAreaAssets {
    mesh: Handle<Mesh>,
    material: Handle<SelectionAreaMaterial>
}

/// Marker component for the drag-selection ground decal entity.
#[derive(Component)]
struct SelectionAreaQuadMarker;

/// Vertical offset applied to the selection-area decal above the terrain hit point, to avoid z-fighting.
const SELECTION_AREA_Y_OFFSET: f32 = 0.03;

/// Demo base: a control room and three domes linked by tunnels, and an airlock opening on a path
/// that serves the outdoor production buildings.
const DEMO_BASE: &[(&str, [f32; 2], GridRotation)] = &[
    // Habitat
    ("Dome Medium", [1.0, 1.0], GridRotation::R0),
    ("Control Room", [1.0, -9.0], GridRotation::R0),
    ("Tunnel Straight", [9.0, 1.0], GridRotation::R0),
    ("Tunnel Straight", [11.0, 1.0], GridRotation::R0),
    ("Tunnel Straight", [13.0, 1.0], GridRotation::R0),
    ("Dome Small", [19.0, 1.0], GridRotation::R0),
    ("Tunnel Straight", [1.0, 9.0], GridRotation::R90),
    ("Tunnel Straight", [1.0, 11.0], GridRotation::R90),
    ("Dome Small", [1.0, 17.0], GridRotation::R0),
    // Way out
    ("Tunnel Straight", [-7.0, 1.0], GridRotation::R0),
    ("Airlock", [-9.0, 1.0], GridRotation::R0),
    ("Path Straight", [-11.0, 1.0], GridRotation::R0),
    // Path spine along x = -13
    ("Path Intersection", [-13.0, -9.0], GridRotation::R0),
    ("Path Intersection", [-13.0, -7.0], GridRotation::R0),
    ("Path Intersection", [-13.0, -5.0], GridRotation::R0),
    ("Path Intersection", [-13.0, -3.0], GridRotation::R0),
    ("Path Intersection", [-13.0, -1.0], GridRotation::R0),
    ("Path Intersection", [-13.0, 1.0], GridRotation::R0),
    ("Path Intersection", [-13.0, 3.0], GridRotation::R0),
    ("Path Intersection", [-13.0, 5.0], GridRotation::R0),
    ("Path Intersection", [-13.0, 7.0], GridRotation::R0),
    ("Path Intersection", [-13.0, 9.0], GridRotation::R0),
    ("Path Intersection", [-13.0, 11.0], GridRotation::R0),
    // Along the spine, toward the base
    ("Solar Array", [-10.0, 4.0], GridRotation::R0),
    ("Solar Array", [-10.0, 8.0], GridRotation::R0),
    ("Sensor Tower", [-11.0, 11.0], GridRotation::R0),
    ("Battery", [-11.0, -1.0], GridRotation::R0),
    ("Oxygen Generator", [-10.0, -4.0], GridRotation::R0),
    ("Fuel Refinery", [-10.0, -8.0], GridRotation::R0),
    // Along the spine, away from the base
    ("Concrete Extractor", [-16.0, 10.0], GridRotation::R0),
    ("Oxygen Tank", [-15.0, 7.0], GridRotation::R0),
    ("Water Extractor", [-16.0, 4.0], GridRotation::R0),
    ("Water Tank", [-15.0, 1.0], GridRotation::R0),
    ("Storage", [-17.0, -3.0], GridRotation::R0),
    ("Landing Pad", [-17.0, -9.0], GridRotation::R0),
    // Modules in the medium dome
    ("Living Quarters", [4.0, 0.0], GridRotation::R0),
    ("Hydroponic Farm", [-2.0, 0.0], GridRotation::R0),
    ("Garden", [1.0, 1.0], GridRotation::R0),
    ("Diner", [1.0, 5.0], GridRotation::R270),
    ("Grocery", [3.0, 5.0], GridRotation::R270),
    ("Residence", [-1.0, 5.0], GridRotation::R270),
    ("Residence", [1.0, -3.0], GridRotation::R90),
    ("Infirmary", [3.0, -3.0], GridRotation::R90),
    // Modules in the north dome
    ("Research Lab", [2.0, 17.0], GridRotation::R0),
    ("Infirmary", [-1.0, 17.0], GridRotation::R180),
    ("Garden", [1.0, 19.0], GridRotation::R0),
    // Modules in the east dome
    ("Residence", [17.0, 1.0], GridRotation::R180),
    ("Residence", [19.0, 3.0], GridRotation::R270),
    ("Grocery", [21.0, -1.0], GridRotation::R0)
];

/// Floor height of the domes.
const DOME_FLOOR: f32 = 0.17;

/// Demo colonists: position and whether they start indoors.
const DEMO_COLONISTS: &[([f32; 3], bool)] = &[
    ([3.0, DOME_FLOOR, 3.0], true),
    ([-1.0, DOME_FLOOR, 3.0], true),
    ([-1.0, DOME_FLOOR, -3.0], true),
    ([1.0, DOME_FLOOR, 3.0], true),
    ([19.0, DOME_FLOOR, -1.0], true),
    ([21.0, DOME_FLOOR, 1.0], true),
    ([-9.0, 0.0, 12.5], false),
    ([-8.0, 0.0, 13.5], false)
];

fn init_scene(
    mut commands: Commands,
    mut placement_queue: ResMut<PlacementQueue>,
    asset_server: Res<AssetServer>
) {
    // Main camera, above the base
    commands.spawn((
        Name::new("Main Camera"),
        Transform::from_xyz(-1.0, 0.0, 3.0),
        ActiveCamera,
        ThirdPersonController {
            distance: 42.0,
            ..Default::default()
        }
    ));

    for (label, [x, z], rotation) in DEMO_BASE {
        placement_queue.place(*label, Vec2::new(*x, *z), *rotation);
    }

    commands.insert_resource(SelectionOutlineMaterial {
        selected: asset_server.add(OutlineMaterial {
            label: "selection-outline".to_string(),
            color: WdeColor::from_srgba(1.0, 0.85, 0.0, 1.0),
            ..Default::default()
        }),
        hovered: asset_server.add(OutlineMaterial {
            label: "hover-outline".to_string(),
            color: WdeColor::from_srgba(1.0, 1.0, 1.0, 0.55),
            thickness: 0.0025,
            ..Default::default()
        })
    });

    commands.insert_resource(SelectionAreaAssets {
        mesh: asset_server.add(PlaneMesh::from("selection-area-quad", 1, Vec3::Y, true)),
        material: asset_server.add(SelectionAreaMaterial {
            label: "selection-area".to_string(),
            color: WdeColor::from_srgba(0.35, 0.85, 1.0, 0.12),
            ..Default::default()
        })
    });

    let global_parent = commands
        .spawn((Name::new("Global Parent"), Transform::default()))
        .id();
    let gltf_asset: Handle<GltfAsset> = asset_server.load("models/entities/human/human.gltf");
    commands.insert_resource(ColonistModels {
        suit: gltf_asset.clone(),
        indoor: asset_server.load("models/entities/human/human_indoor.gltf")
    });
    // Their model follows `Indoors` (see `swap_colonist_model`)
    for (i, (position, indoors)) in DEMO_COLONISTS.iter().enumerate() {
        commands.spawn((
            Name::new(format!("Colonist {}", i)),
            Transform::from_translation(Vec3::from(*position)),
            Indoors(*indoors),
            PendingHumanCollider(gltf_asset.clone()),
            EntityMarker,
            AnimationPlayer::default(),
            WalkTracker::default(),
            NavAgent::default(),
            ChildOf(global_parent)
        ));
    }
}

/// Adds a box collider to the human entity once its glTF asset has finished loading,
/// sized and offset from the model's bounding box.
fn add_human_collider(
    mut commands: Commands,
    gltf_assets: Res<Assets<GltfAsset>>,
    query: Query<(Entity, &PendingHumanCollider)>
) {
    for (entity, pending) in &query {
        let Some(gltf_asset) = gltf_assets.get(&pending.0) else {
            continue;
        };

        let bbox = &gltf_asset.bbox;
        let bbox_extent = bbox.max - bbox.min;
        let bbox_offset = bbox.min + bbox_extent / 2.0;
        let pick_extent = bbox_extent * Vec3::new(COLONIST_PICK_MARGIN, 1.0, COLONIST_PICK_MARGIN);

        commands.entity(entity).remove::<PendingHumanCollider>();
        commands.spawn((
            Name::new("Human Collider"),
            Transform::from_translation(bbox_offset),
            Collider::from(BoxCollider::new(pick_extent).with_group(COLONIST_COLLIDER_GROUP)),
            ChildOf(entity)
        ));
    }
}

#[derive(Component)]
pub(crate) struct EntitySelectedMarker;

/// Drag state of the selection: where the press started, on screen and on the terrain.
#[derive(Default)]
struct SelectionDrag {
    press: Option<(Vec2, Vec3)>,
    decal: Option<Entity>
}

/// Colonist under the cursor, through the colonist picking colliders.
fn colonist_under_cursor(
    window: &Window,
    camera: &Query<(&GlobalTransform, &CameraView), With<Camera>>,
    physics: &PhysicsWorld,
    colliders: &Query<&ChildOf, With<Collider>>
) -> Option<Entity> {
    let cursor = window.cursor_position()?;
    let (camera_transform, camera_view) = camera.single().ok()?;
    let aspect_ratio = window.size().x / window.size().y;
    let ray = Ray::from_ndc(
        cursor / window.size(),
        aspect_ratio,
        camera_transform,
        camera_view
    );
    let (collider, _) =
        physics.cast_ray(&ray, &RayCastConfig::with_filter(COLONIST_COLLIDER_GROUP))?;
    colliders
        .get(collider)
        .ok()
        .map(|child_of| child_of.parent())
}

/// Selects colonists: click one (Shift to add / remove), drag a rectangle on the ground, or
/// click the ground to clear. Hovered colonists get a light outline.
#[allow(clippy::too_many_arguments)]
fn select_entity(
    mut commands: Commands,
    mut drag: Local<SelectionDrag>,
    mut hovered: Local<Option<Entity>>,
    (mouse_input, keys, cursor_pos, input_capture, ui_pointer): (
        Res<ButtonInput<MouseButton>>,
        Res<ButtonInput<KeyCode>>,
        Res<TerrainCursorPos>,
        Res<PlacementInputCapture>,
        Res<UiPointer>
    ),
    (window, camera, physics, colliders): (
        Single<&Window, With<PrimaryWindow>>,
        Query<(&GlobalTransform, &CameraView), With<Camera>>,
        Res<PhysicsWorld>,
        Query<&ChildOf, With<Collider>>
    ),
    outlines: Res<SelectionOutlineMaterial>,
    selection_area_assets: Res<SelectionAreaAssets>,
    selected_entities: Query<Entity, With<EntitySelectedMarker>>,
    colonists: Query<(Entity, &Transform), (With<EntityMarker>, Without<SelectionAreaQuadMarker>)>,
    children_query: Query<&Children>,
    mesh_query: Query<(), With<Mesh3d>>,
    mut quad_transform_query: Query<&mut Transform, With<SelectionAreaQuadMarker>>
) {
    let set_outline =
        |commands: &mut Commands, entity: Entity, material: Option<&Handle<OutlineMaterial>>| {
            set_entity_outline(commands, &children_query, &mesh_query, entity, material);
        };

    // Hover: light outline on the colonist under the cursor (selected ones keep theirs)
    let under_cursor =
        match input_capture.is_captured() || ui_pointer.over_ui || drag.decal.is_some() {
            true => None,
            false => colonist_under_cursor(&window, &camera, &physics, &colliders)
                .filter(|e| colonists.contains(*e))
        };
    if *hovered != under_cursor {
        if let Some(previous) = *hovered
            && !selected_entities.contains(previous)
        {
            set_outline(&mut commands, previous, None);
        }
        if let Some(entity) = under_cursor
            && !selected_entities.contains(entity)
        {
            set_outline(&mut commands, entity, Some(&outlines.hovered));
        }
        *hovered = under_cursor;
    }

    // Placement tools own the mouse: cancel any drag in progress and stand by
    if input_capture.tool {
        drag.press = None;
        if let Some(decal) = drag.decal.take() {
            commands.entity(decal).despawn();
        }
        return;
    }

    let Some(cursor_screen) = window.cursor_position() else {
        return;
    };
    if mouse_input.just_pressed(MouseButton::Left) && !input_capture.over_ui && !ui_pointer.over_ui
    {
        drag.press = Some((cursor_screen, cursor_pos.pos_or_last()));
        return;
    }
    let Some((press_screen, press_world)) = drag.press else {
        return;
    };
    let is_drag = drag.decal.is_some() || press_screen.distance(cursor_screen) > DRAG_THRESHOLD;
    let end_world = cursor_pos.pos_or_last();
    let (min, max) = (press_world.min(end_world), press_world.max(end_world));

    if mouse_input.just_released(MouseButton::Left) {
        drag.press = None;
        if let Some(decal) = drag.decal.take() {
            commands.entity(decal).despawn();
        }
        let additive = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);

        // Colonists picked by this click or drag
        let picked: Vec<Entity> = match is_drag {
            true => {
                let rect = Rect::new(min.x, min.z, max.x, max.z);
                colonists
                    .iter()
                    .filter(|(_, t)| rect.contains(t.translation.xz()))
                    .map(|(e, _)| e)
                    .collect()
            }
            false => under_cursor.into_iter().collect()
        };

        // A plain click on a selected colonist with Shift removes it from the selection
        if additive
            && !is_drag
            && let Some(&entity) = picked.first()
            && selected_entities.contains(entity)
        {
            commands.entity(entity).remove::<EntitySelectedMarker>();
            set_outline(&mut commands, entity, Some(&outlines.hovered));
            return;
        }
        if !additive {
            for entity in &selected_entities {
                if !picked.contains(&entity) {
                    commands.entity(entity).remove::<EntitySelectedMarker>();
                    set_outline(&mut commands, entity, None);
                }
            }
        }
        for entity in picked {
            commands.entity(entity).insert(EntitySelectedMarker);
            set_outline(&mut commands, entity, Some(&outlines.selected));
        }
        return;
    }

    // Dragging: show the ground decal over the rectangle
    if is_drag {
        let decal = *drag.decal.get_or_insert_with(|| {
            commands
                .spawn((
                    Name::new("Selection Area Decal"),
                    SelectionAreaQuadMarker,
                    Mesh3d(selection_area_assets.mesh.clone()),
                    PbrMaterial3d(selection_area_assets.material.clone()),
                    Transform::from_scale(Vec3::new(0.0, 1.0, 0.0)),
                    PbrSsboTransformUuid::default()
                ))
                .id()
        });
        if let Ok(mut transform) = quad_transform_query.get_mut(decal) {
            let center = (min + max) * 0.5;
            transform.translation =
                Vec3::new(center.x, center.y + SELECTION_AREA_Y_OFFSET, center.z);
            transform.scale = Vec3::new(
                (max.x - min.x).max(f32::EPSILON),
                1.0,
                (max.z - min.z).max(f32::EPSILON)
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
    outline_material: Option<&Handle<OutlineMaterial>>
) {
    for child in children_query.iter_descendants(entity) {
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

/// Swaps colonists between their suit and indoor models when they cross an airlock.
fn swap_colonist_model(
    mut commands: Commands,
    models: Res<ColonistModels>,
    changed: Query<(Entity, &Indoors, Option<&Children>), (Changed<Indoors>, With<EntityMarker>)>,
    model_parts: Query<(), Or<(With<Mesh3d>, With<AnimationBone>)>>,
    mut gltf_spawn_queue: ResMut<GltfSpawnQueue>
) {
    for (entity, indoors, children) in &changed {
        for child in children.into_iter().flatten() {
            if model_parts.contains(*child) {
                commands.entity(*child).despawn();
            }
        }
        let model = match indoors.0 {
            true => models.indoor.clone(),
            false => models.suit.clone()
        };
        GltfLoader::spawn(&mut gltf_spawn_queue, model, entity);
    }
}

/// Smoothed walking speed of a colonist, measured from its movement.
#[derive(Component, Default)]
pub(crate) struct WalkTracker {
    last_position: Option<Vec3>,
    pub(crate) speed: f32
}

/// Speed under which a colonist counts as standing still.
pub(crate) const WALK_MIN_SPEED: f32 = 0.05;
/// Playback speed range of the walk cycle: beyond it, feet slide rather than the legs blur.
const WALK_PLAYBACK_RANGE: (f32, f32) = (0.5, 2.2);

/// Plays the walk cycle at the colonist's speed while it moves, idles otherwise.
fn animate_colonists(
    models: Res<ColonistModels>,
    gltf_assets: Res<Assets<GltfAsset>>,
    clips: Res<Assets<AnimationClip>>,
    mut colonists: Query<(&Transform, &mut WalkTracker, &mut AnimationPlayer)>,
    time: Res<Time>
) {
    // Both colonist models share the same rig: their clips are interchangeable
    let Some(model) = gltf_assets.get(&models.suit) else {
        return;
    };
    let (Some(walk), Some(idle)) = (model.animation("Walk"), model.animation("Idle")) else {
        return;
    };
    let stride_speed = clips.get(walk).and_then(|c| c.ground_speed).unwrap_or(1.0);
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }

    for (transform, mut tracker, mut player) in &mut colonists {
        let position = transform.translation;
        let moved = tracker
            .last_position
            .map_or(0.0, |last| (position - last).xz().length());
        tracker.last_position = Some(position);
        tracker.speed += (moved / dt - tracker.speed) * (dt * 12.0).min(1.0);

        if tracker.speed > WALK_MIN_SPEED {
            let (min, max) = WALK_PLAYBACK_RANGE;
            player
                .crossfade(walk.clone(), 0.2)
                .set_speed((tracker.speed / stride_speed).clamp(min, max));
        } else {
            player.crossfade(idle.clone(), 0.3).set_speed(1.0);
        }
    }
}

/// Keeps the selection outline on meshes spawned under a selected entity (e.g. model swaps).
fn outline_new_meshes(
    mut commands: Commands,
    outline_material: Res<SelectionOutlineMaterial>,
    new_meshes: Query<Entity, Added<Mesh3d>>,
    parents: Query<&ChildOf>,
    selected: Query<(), With<EntitySelectedMarker>>
) {
    for mesh in &new_meshes {
        if parents.iter_ancestors(mesh).any(|e| selected.contains(e)) {
            commands
                .entity(mesh)
                .insert(PbrMaterial3d(outline_material.selected.clone()));
        }
    }
}

fn set_entities_target(
    selected_entities: Query<Entity, With<EntitySelectedMarker>>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    cursor_pos: Res<TerrainCursorPos>,
    mut navigator: ResMut<TerrainNavigator>
) {
    if selected_entities.is_empty() || !mouse_input.just_pressed(MouseButton::Right) {
        return;
    }
    navigator.add_group(selected_entities, cursor_pos.pos_or_last());
}
