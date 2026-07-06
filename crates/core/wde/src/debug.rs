//! Debug visualization for engine systems that don't have their own dedicated editor panel.
//! Currently only exposes a "Show Colliders" toggle that draws box colliders as gizmo cubes.
use bevy::prelude::*;
use wde_editor::prelude::*;
use wde_gizmos::prelude::*;
use wde_physics::prelude::*;
use wde_renderer::prelude::Color;

/// Toggles for physics debug visualizations, editable from the "Engine/Physics" editor panel.
#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
pub struct PhysicsDebugSettings {
    pub show_colliders: bool
}

pub(crate) struct PhysicsDebugPlugin;
impl Plugin for PhysicsDebugPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PhysicsDebugSettings>()
            .register_type::<PhysicsDebugSettings>()
            .add_systems(Update, (edit_physics_debug_settings, draw_collider_gizmos));
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
        .open(ui_menu.clicked_mut("Engine/Physics"))
        .show(&ctx.0, |ui| {
            ui.add(Checkbox::new(&mut settings.show_colliders, "Show Colliders"));
        });
}

/// Draws every box collider as a gizmo cube, positioned like the physics engine sees it: at the
/// entity's world translation only, ignoring rotation and scale (colliders are always
/// axis-aligned, see [`wde_physics`] `handle_changes`).
fn draw_collider_gizmos(
    settings: Res<PhysicsDebugSettings>,
    colliders: Query<(&GlobalTransform, &Collider)>,
    mut gizmos: ResMut<Gizmos>
) {
    if !settings.show_colliders {
        return;
    }

    for (transform, collider) in &colliders {
        let Some(half_extents) = collider.box_half_extents() else {
            continue;
        };

        let cube_transform =
            Transform::from_translation(transform.translation()).with_scale(half_extents * 2.0);
        gizmos.cube(cube_transform, Color::from_srgba(0.0, 1.0, 0.0, 1.0));
    }
}
