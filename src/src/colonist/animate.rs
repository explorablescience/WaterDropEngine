use wde::prelude::*;

use bevy::prelude::*;

use crate::colonist::spawn::ColonistModels;

/// Speed under which a colonist counts as standing still.
const WALK_MIN_SPEED: f32 = 0.05;
/// Playback speed range of the walk cycle: beyond it, feet slide rather than the legs blur.
const WALK_PLAYBACK_RANGE: (f32, f32) = (0.5, 2.2);

pub struct AnimateColonistPlugin;
impl Plugin for AnimateColonistPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<WalkTracker>()
            .add_systems(Update, animate_colonists);
    }
}

/// Smoothed walking speed of a colonist, measured from its movement.
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub struct WalkTracker {
    last_position: Option<Vec3>,
    pub speed: f32
}

/// Plays the walk cycle at the colonist's speed while it moves, idles otherwise.
fn animate_colonists(
    models: Res<ColonistModels>,
    gltf_assets: Res<Assets<GltfAsset>>,
    clips: Res<Assets<AnimationClip>>,
    mut colonists: Query<(&Transform, &mut WalkTracker, &mut AnimationPlayer)>,
    time: Res<Time>
) {
    // Both colonist models share same rig
    let Some(model) = gltf_assets.get(&models.suit) else {
        return;
    };
    let (Some(walk), Some(idle)) = (model.animation("Walk"), model.animation("Idle")) else {
        return;
    };
    let stride_speed = clips.get(walk).and_then(|c| c.ground_speed).unwrap_or(1.0);
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return; // Avoid division by zero
    }

    // Update each colonist's walk tracker and animation player
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
