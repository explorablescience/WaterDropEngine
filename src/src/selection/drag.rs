use wde::prelude::*;

use bevy::{prelude::*, window::PrimaryWindow};

use crate::colonist::ColonistMarker;

/// Cursor travel (in pixels) beyond which a press becomes a drag selection.
const DRAG_THRESHOLD: f32 = 6.0;
/// Vertical offset of the selection-area decal above the terrain, to avoid z-fighting.
const SELECTION_AREA_Y_OFFSET: f32 = 0.03;

pub struct DragPlugin;
impl Plugin for DragPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SelectionDrag>()
            .register_type::<SelectionDrag>()
            .register_type::<DragRelease>()
            .register_type::<SelectionAreaAssets>()
            .register_type::<SelectionAreaDecal>()
            .add_systems(Startup, init_selection_area);
    }
}

/// How the left button was released this frame.
#[derive(Reflect)]
pub enum DragRelease {
    Click,
    /// Colonists inside the dragged area.
    Area(Vec<Entity>)
}

/// Drag state of the selection: where the press started, on screen and on the terrain.
#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
pub struct SelectionDrag {
    press: Option<(Vec2, Vec3)>,
    decal: Option<Entity>,
    pub release: Option<DragRelease>
}

/// The mesh and translucent material of the drag-selection ground decal.
#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub(super) struct SelectionAreaAssets {
    mesh: Handle<Mesh>,
    material: Handle<SelectionAreaMaterial>
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub(super) struct SelectionAreaDecal;

fn init_selection_area(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(SelectionAreaAssets {
        mesh: asset_server.add(PlaneMesh::from("selection-area-quad", 1, Vec3::Y, true)),
        material: asset_server.add(SelectionAreaMaterial {
            label: "selection-area".to_string(),
            color: WdeColor::from_srgba(0.3, 0.95, 0.4, 0.15),
            ..Default::default()
        })
    });
}

/// Tracks the left button: a press then release is a click, unless the cursor moved enough in
/// between to drag a rectangle on the ground, shown by a decal.
pub(super) fn update_drag(
    mut commands: Commands,
    mut drag: ResMut<SelectionDrag>,
    (mouse_input, cursor_pos, input_capture, ui_pointer): (
        Res<ButtonInput<MouseButton>>,
        Res<TerrainCursorPos>,
        Res<PlacementInputCapture>,
        Res<UiPointer>
    ),
    window: Single<&Window, With<PrimaryWindow>>,
    assets: Res<SelectionAreaAssets>,
    colonists: Query<(Entity, &GlobalTransform), With<ColonistMarker>>,
    mut decals: Query<&mut Transform, With<SelectionAreaDecal>>
) {
    drag.release = None;

    // Placement tools and UI own the mouse: cancel any drag in progress
    let captured = input_capture.is_captured() || ui_pointer.over_ui;
    let Some(cursor_screen) = window.cursor_position().filter(|_| !captured) else {
        drag.press = None;
        if let Some(decal) = drag.decal.take() {
            commands.entity(decal).despawn();
        }
        return;
    };
    if mouse_input.just_pressed(MouseButton::Left) {
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
        drag.release = Some(match is_drag {
            true => {
                let area = Rect::new(min.x, min.z, max.x, max.z);
                DragRelease::Area(
                    colonists
                        .iter()
                        .filter(|(_, transform)| area.contains(transform.translation().xz()))
                        .map(|(entity, _)| entity)
                        .collect()
                )
            }
            false => DragRelease::Click
        });
        return;
    }

    if is_drag {
        let decal = *drag.decal.get_or_insert_with(|| {
            commands
                .spawn((
                    Name::new("Selection Area Decal"),
                    SelectionAreaDecal,
                    Mesh3d(assets.mesh.clone()),
                    PbrMaterial3d(assets.material.clone()),
                    Transform::from_scale(Vec3::new(0.0, 1.0, 0.0)),
                    PbrSsboTransformUuid::default()
                ))
                .id()
        });
        if let Ok(mut transform) = decals.get_mut(decal) {
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
