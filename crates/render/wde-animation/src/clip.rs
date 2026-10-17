use std::collections::HashMap;

use bevy::prelude::*;

/// How values are interpolated between two keyframes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Interpolation {
    /// Holds the value of the previous keyframe.
    Step,
    Linear
}

/// Values over time, sorted by time.
#[derive(Clone, Debug)]
pub struct Keyframes<T> {
    pub times: Vec<f32>,
    pub values: Vec<T>,
    pub interpolation: Interpolation
}

/// Value type that can be interpolated between keyframes.
pub trait Interpolate: Copy {
    fn interpolate(from: Self, to: Self, t: f32) -> Self;
}
impl Interpolate for Vec3 {
    fn interpolate(from: Self, to: Self, t: f32) -> Self {
        from.lerp(to, t)
    }
}
impl Interpolate for Quat {
    fn interpolate(from: Self, to: Self, t: f32) -> Self {
        from.slerp(to, t)
    }
}

impl<T: Interpolate> Keyframes<T> {
    /// Value at `time`, held constant before the first and after the last keyframe.
    pub fn sample(&self, time: f32) -> Option<T> {
        let count = self.times.len().min(self.values.len());
        if count == 0 {
            return None;
        }
        let next = self.times[..count].partition_point(|&t| t <= time);
        if next == 0 {
            return Some(self.values[0]);
        }
        if next == count {
            return Some(self.values[count - 1]);
        }
        let (from, to) = (self.times[next - 1], self.times[next]);
        Some(match self.interpolation {
            Interpolation::Step => self.values[next - 1],
            Interpolation::Linear => T::interpolate(
                self.values[next - 1],
                self.values[next],
                (time - from) / (to - from).max(f32::EPSILON)
            )
        })
    }
}

/// Animation of one bone. Missing channels keep the bone's rest value.
#[derive(Clone, Debug, Default)]
pub struct BoneTrack {
    pub translation: Option<Keyframes<Vec3>>,
    pub rotation: Option<Keyframes<Quat>>,
    pub scale: Option<Keyframes<Vec3>>
}
impl BoneTrack {
    /// Local transform of the bone at `time`.
    pub fn sample(&self, time: f32, rest: &Transform) -> Transform {
        let sample = |keys: &Option<Keyframes<Vec3>>, default| {
            keys.as_ref()
                .and_then(|k| k.sample(time))
                .unwrap_or(default)
        };
        Transform {
            translation: sample(&self.translation, rest.translation),
            rotation: self
                .rotation
                .as_ref()
                .and_then(|k| k.sample(time))
                .unwrap_or(rest.rotation),
            scale: sample(&self.scale, rest.scale)
        }
    }
}

/// Keyframed animation of a set of bones, identified by name.
#[derive(Asset, TypePath, Clone, Debug, Default)]
pub struct AnimationClip {
    pub name: String,
    /// Time of the last keyframe, in seconds.
    pub duration: f32,
    pub tracks: HashMap<String, BoneTrack>,
    /// Ground speed the clip was authored for (glTF extras `ground_speed`), to match the playback
    /// speed to the movement speed.
    pub ground_speed: Option<f32>
}
