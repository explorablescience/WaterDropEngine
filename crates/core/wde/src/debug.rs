//! Debug visualization for engine systems that don't have their own dedicated editor panel.
//! Exposes a "Show Colliders" toggle that draws box colliders as gizmo cubes, a "Show Frustums"
//! toggle that draws the frustum of the cameras, and draws the PBR cull groups when their
//! "Show Groups" culling setting is enabled.
use bevy::{prelude::*, window::PrimaryWindow};
use wde_camera::prelude::*;
use wde_editor::prelude::*;
use wde_gizmos::prelude::*;
use wde_physics::prelude::*;
use wde_renderer::prelude::WdeColor;

/// Toggles for physics debug visualizations, editable from the "Engine/Physics" editor panel.
#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
pub struct PhysicsDebugSettings {
    pub show_colliders: bool,
    pub show_heightfields: bool
}

pub(crate) struct PhysicsDebugPlugin;
impl Plugin for PhysicsDebugPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PhysicsDebugSettings>()
            .register_type::<PhysicsDebugSettings>()
            .add_systems(Update, (edit_physics_debug_settings, draw_collider_gizmos));
        #[cfg(feature = "pbr")]
        app.add_systems(Update, draw_cull_group_gizmos);
    }
}

fn edit_physics_debug_settings(
    ctx: Res<UIContext>,
    mut settings: ResMut<PhysicsDebugSettings>,
    mut ui_menu: ResMut<UIMenu>
) {
    UIWindow::new("Physics")
        .resizable(false)
        .default_pos([100.0, 260.0])
        .open(ui_menu.clicked_mut("Debug/Physics"))
        .show(&ctx.0, |ui| {
            ui.add(Checkbox::new(
                &mut settings.show_colliders,
                "Show Colliders"
            ));
            ui.add(Checkbox::new(
                &mut settings.show_heightfields,
                "Show Heightfields"
            ));
        });
}

/// Draws every collider as a gizmo wireframe, positioned like the physics engine sees it: at the
/// entity's world translation only, ignoring rotation and scale (colliders are always
/// axis-aligned, see [`wde_physics`] `handle_changes`).
fn draw_collider_gizmos(
    settings: Res<PhysicsDebugSettings>,
    colliders: Query<(&GlobalTransform, &Collider)>,
    mut gizmos: ResMut<Gizmos>
) {
    if !settings.show_colliders && !settings.show_heightfields {
        return;
    }

    for (transform, collider) in &colliders {
        let origin = transform.translation();

        if settings.show_colliders
            && let Some(half_extents) = collider.box_half_extents()
        {
            let cube_transform = Transform::from_translation(origin).with_scale(half_extents * 2.0);
            gizmos.cube(cube_transform, WdeColor::from_srgba(0.0, 1.0, 0.0, 1.0));
        }
        if settings.show_heightfields
            && let Some(lines) = collider.heightfield_wireframe()
        {
            for (start, end) in lines {
                gizmos.line(
                    origin + start,
                    origin + end,
                    WdeColor::from_srgba(0.2, 0.6, 1.0, 1.0)
                );
            }
        }
    }
}

#[cfg(feature = "pbr")]
fn draw_cull_group_gizmos(
    settings: Res<wde_pbr::prelude::CullingSettings>,
    groups: Query<&wde_pbr::prelude::CullGroup>,
    mut gizmos: ResMut<Gizmos>
) {
    if !settings.show_groups {
        return;
    }
    for group in &groups {
        if group.0.is_empty() {
            continue;
        }
        let transform =
            Transform::from_translation(group.0.center()).with_scale(group.0.half_extents() * 2.0);
        gizmos.cube(transform, WdeColor::from_srgba(1.0, 0.6, 0.0, 1.0));
    }
}

/// Toggles for camera debug visualizations, editable from the "Debug/Cameras" editor panel.
#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct CameraDebugSettings {
    pub show_frustums: bool,
    /// Distance the frustums are drawn up to, when nearer than their far plane.
    pub frustum_length: f32
}
impl Default for CameraDebugSettings {
    fn default() -> Self {
        Self {
            show_frustums: false,
            frustum_length: 100.0
        }
    }
}

pub(crate) struct CameraDebugPlugin;
impl Plugin for CameraDebugPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraDebugSettings>()
            .register_type::<CameraDebugSettings>()
            .add_systems(Update, (edit_camera_debug_settings, draw_camera_frustums));
    }
}

const CULLING_FRUSTUM_COLOR: WdeColor = WdeColor::Srgba(1.0, 0.3, 0.1, 1.0);
const FRUSTUM_COLOR: WdeColor = WdeColor::Srgba(0.2, 0.8, 1.0, 1.0);

fn edit_camera_debug_settings(
    ctx: Res<UIContext>,
    mut settings: ResMut<CameraDebugSettings>,
    mut ui_menu: ResMut<UIMenu>
) {
    UIWindow::new("Camera Debug")
        .resizable(false)
        .default_pos([100.0, 260.0])
        .open(ui_menu.clicked_mut("Debug/Cameras"))
        .show(&ctx.0, |ui| {
            ui.add(Checkbox::new(&mut settings.show_frustums, "Show Frustums"));
            ui.add(
                Slider::new(&mut settings.frustum_length, 1.0..=1000.0)
                    .logarithmic(true)
                    .text("Frustum Length")
            );
            ui.label("Red: culling camera, blue: others. The rendered camera's is hidden.");
        });
}

/// Draws the frustum of every camera but the rendered one, which would lie on the screen edges.
fn draw_camera_frustums(
    settings: Res<CameraDebugSettings>,
    cameras: Query<(&GlobalTransform, &CameraView, Has<CullingCamera>), Without<ActiveCamera>>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut gizmos: ResMut<Gizmos>
) {
    if !settings.show_frustums {
        return;
    }
    let aspect = window
        .single()
        .map_or(1.0, |w| w.width() / w.height().max(1.0));
    for (transform, view, culling) in &cameras {
        let color = match culling {
            true => CULLING_FRUSTUM_COLOR,
            false => FRUSTUM_COLOR
        };
        // Cameras look down -Z
        let corners = |depth: f32| {
            let half_height = depth * (view.fov.to_radians() * 0.5).tan();
            let half_width = half_height * aspect;
            [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)].map(|(x, y)| {
                transform.transform_point(Vec3::new(x * half_width, y * half_height, -depth))
            })
        };
        let near = corners(view.znear);
        let far = corners(view.zfar.min(settings.frustum_length));
        for i in 0..4 {
            let next = (i + 1) % 4;
            gizmos.line(near[i], near[next], color);
            gizmos.line(far[i], far[next], color);
            gizmos.line(near[i], far[i], color);
        }
    }
}
