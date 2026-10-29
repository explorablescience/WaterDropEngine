use bevy::prelude::*;
use wde::prelude::*;

mod demo_base;
mod demo_colonist;

pub struct DemoPlugin;
impl Plugin for DemoPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Startup,
            (
                spawn_initial_camera,
                demo_base::spawn_demo_base,
                demo_colonist::spawn_demo_colonists
            )
        );
    }
}

/// Spawns the game camera, holding every camera role, and a free flying editor camera. Roles are
/// switched from the `Camera/Cameras` editor window.
fn spawn_initial_camera(mut commands: Commands) {
    commands.spawn((
        Name::new("Game Camera"),
        Transform::from_xyz(-1.0, 0.0, 3.0),
        ActiveCamera,
        CullingCamera,
        ControlledCamera,
        ThirdPersonController {
            distance: 42.0,
            ..Default::default()
        }
    ));
    commands.spawn((
        Name::new("Editor Camera"),
        Transform::from_xyz(-60.0, 70.0, 60.0).looking_at(Vec3::ZERO, Vec3::Y),
        Camera,
        FreeCameraController {
            // Left click stays for selection
            mouse_key_cursor_grab: MouseButton::Right,
            ..Default::default()
        }
    ));
}
