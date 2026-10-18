use bevy::prelude::*;

use crate::clip::AnimationClip;

/// Rigid part of an animated model, posed by the [`AnimationPlayer`] of its closest ancestor.
#[derive(Component, Clone, Debug)]
pub struct AnimationBone {
    pub name: String,
    /// Local transform when no clip animates the bone.
    pub rest: Transform
}

/// Player animating a bone.
#[derive(Component)]
pub(crate) struct BoneOf(Entity);

/// A clip being played.
#[derive(Clone, Debug)]
struct PlayingClip {
    clip: Handle<AnimationClip>,
    time: f32,
    speed: f32,
    /// Loops, or stops on its last pose.
    repeat: bool,
    finished: bool
}
impl PlayingClip {
    fn new(clip: Handle<AnimationClip>, speed: f32, repeat: bool) -> Self {
        Self {
            clip,
            time: 0.0,
            speed,
            repeat,
            finished: false
        }
    }
}

/// Plays clips (looping, or once) on the [`AnimationBone`]s below its entity, blending smoothly
/// from one clip to the next.
#[derive(Component, Clone, Debug, Default)]
pub struct AnimationPlayer {
    active: Option<PlayingClip>,
    /// Previous clip, fading out.
    previous: Option<PlayingClip>,
    fade: f32,
    fade_duration: f32
}
impl AnimationPlayer {
    /// Plays `clip` right away, from its start.
    pub fn play(&mut self, clip: Handle<AnimationClip>) -> &mut Self {
        self.active = Some(PlayingClip::new(clip, 1.0, true));
        self.previous = None;
        self
    }

    /// Blends to `clip` over `fade` seconds and plays it once, from its start (even if it is
    /// already playing), holding its last pose. See [`Self::is_finished`].
    pub fn play_once(&mut self, clip: Handle<AnimationClip>, fade: f32) -> &mut Self {
        self.previous = self.active.take();
        self.active = Some(PlayingClip::new(clip, 1.0, false));
        self.fade = 0.0;
        self.fade_duration = fade;
        self
    }

    /// Whether the clip played once is over.
    pub fn is_finished(&self) -> bool {
        self.active.as_ref().is_some_and(|a| a.finished)
    }

    /// Blends from the current clip to `clip` over `duration` seconds. Keeps playing if `clip`
    /// is already the active clip.
    pub fn crossfade(&mut self, clip: Handle<AnimationClip>, duration: f32) -> &mut Self {
        if self.is_playing(&clip) {
            return self;
        }
        let speed = self.active.as_ref().map_or(1.0, |a| a.speed);
        self.previous = self.active.take();
        self.active = Some(PlayingClip::new(clip, speed, true));
        self.fade = 0.0;
        self.fade_duration = duration;
        self
    }

    /// Playback speed of the active clip (1 = authored speed).
    pub fn set_speed(&mut self, speed: f32) -> &mut Self {
        if let Some(active) = &mut self.active {
            active.speed = speed;
        }
        self
    }

    /// Stops animating: the bones go back to their rest pose.
    pub fn stop(&mut self) {
        self.active = None;
        self.previous = None;
    }

    pub fn is_playing(&self, clip: &Handle<AnimationClip>) -> bool {
        self.active.as_ref().is_some_and(|a| a.clip == *clip)
    }

    /// Weight of the active clip against the previous one.
    fn blend(&self) -> f32 {
        match self.previous {
            Some(_) => (self.fade / self.fade_duration.max(f32::EPSILON)).min(1.0),
            None => 1.0
        }
    }
}

/// Links new bones to the player of their closest ancestor.
pub(crate) fn link_bones(
    mut commands: Commands,
    bones: Query<Entity, (With<AnimationBone>, Without<BoneOf>)>,
    parents: Query<&ChildOf>,
    players: Query<(), With<AnimationPlayer>>
) {
    for bone in &bones {
        if let Some(player) = parents.iter_ancestors(bone).find(|e| players.contains(*e)) {
            commands.entity(bone).insert(BoneOf(player));
        }
    }
}

pub(crate) fn advance_players(
    mut players: Query<&mut AnimationPlayer>,
    clips: Res<Assets<AnimationClip>>,
    time: Res<Time>
) {
    let dt = time.delta_secs();
    for mut player in &mut players {
        let player = player.as_mut();
        for playing in [&mut player.active, &mut player.previous]
            .into_iter()
            .flatten()
        {
            let duration = clips.get(&playing.clip).map_or(0.0, |c| c.duration);
            if duration <= 0.0 {
                playing.time = 0.0;
            } else if playing.repeat {
                playing.time = (playing.time + dt * playing.speed).rem_euclid(duration);
            } else {
                playing.time = (playing.time + dt * playing.speed).min(duration);
                playing.finished = playing.time >= duration;
            }
        }
        if player.previous.is_some() {
            player.fade += dt;
            if player.fade >= player.fade_duration {
                player.previous = None;
            }
        }
    }
}

pub(crate) fn pose_bones(
    mut bones: Query<(&AnimationBone, &BoneOf, &mut Transform)>,
    players: Query<&AnimationPlayer>,
    clips: Res<Assets<AnimationClip>>
) {
    for (bone, bone_of, mut transform) in &mut bones {
        let Ok(player) = players.get(bone_of.0) else {
            continue;
        };
        let pose = |playing: &Option<PlayingClip>| {
            playing.as_ref().map(|p| {
                clips
                    .get(&p.clip)
                    .and_then(|c| c.tracks.get(&bone.name))
                    .map_or(bone.rest, |track| track.sample(p.time, &bone.rest))
            })
        };
        let active = pose(&player.active).unwrap_or(bone.rest);
        let posed = match pose(&player.previous) {
            Some(previous) => {
                let t = player.blend();
                Transform {
                    translation: previous.translation.lerp(active.translation, t),
                    rotation: previous.rotation.slerp(active.rotation, t),
                    scale: previous.scale.lerp(active.scale, t)
                }
            }
            None => active
        };
        transform.set_if_neq(posed);
    }
}
