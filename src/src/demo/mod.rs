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

fn spawn_initial_camera(mut commands: Commands) {
    commands.spawn((
        Name::new("Main Camera"),
        Transform::from_xyz(-1.0, 0.0, 3.0),
        ActiveCamera,
        ThirdPersonController {
            distance: 42.0,
            ..Default::default()
        }
    ));
}
