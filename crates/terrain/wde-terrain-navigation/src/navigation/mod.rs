use bevy::prelude::*;

use crate::TargetLocation;

pub struct NavigationPlugin;
impl Plugin for NavigationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, move_entities_towards_target);
    }
}

fn move_entities_towards_target(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &TargetLocation)>,
    time: Res<Time>
) {
    for (entity, mut transform, target_location) in &mut query {
        let direction = (target_location.0 - transform.translation).normalize_or_zero();
        let speed = 2.0; // units per second
        let delta_move = direction * speed * time.delta_secs();
        transform.translation += delta_move;
        transform.rotation = Quat::from_rotation_arc(Vec3::Z, direction) * Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2);

        // Check if the entity has reached the target location
        if transform.translation.distance(target_location.0) < 0.1 {
            commands.entity(entity).remove::<TargetLocation>();
        }
    }
}
