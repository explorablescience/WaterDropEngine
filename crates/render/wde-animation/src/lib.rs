//! Rigid-part animation for WaterDropEngine.
//!
//! A model is animated by moving rigid parts: each part hangs under an [`AnimationBone`] entity,
//! whose transform is driven by an [`AnimationClip`] (one keyframe track per bone name). An
//! [`AnimationPlayer`] on the root of the hierarchy plays and crossfades clips onto every bone
//! below it.
//!
//! # Example
//! glTF models with `%B%<bone>` nodes spawn their bones automatically, and expose their clips:
//! ```rust
//! fn walk(mut players: Query<&mut AnimationPlayer>, gltf: Res<Assets<GltfAsset>>, model: Res<Model>) {
//!     let walk = gltf.get(&model.0).unwrap().animation("Walk").unwrap();
//!     for mut player in &mut players {
//!         player.crossfade(walk.clone(), 0.2).set_speed(1.5);
//!     }
//! }
//! ```

use bevy::prelude::*;

mod clip;
mod player;

pub use clip::{AnimationClip, BoneTrack, Interpolation, Keyframes};
pub use player::{AnimationBone, AnimationPlayer};

#[doc(hidden)]
pub mod prelude {
    pub use crate::{AnimationBone, AnimationClip, AnimationPlayer, AnimationSystems};
}

/// Systems advancing the players and posing the bones, in [`Update`].
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct AnimationSystems;

pub struct AnimationPlugin;
impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<AnimationClip>()
            .register_type::<AnimationPlayer>()
            .register_type::<AnimationBone>()
            .register_type::<player::BoneOf>()
            .add_systems(
                Update,
                (
                    player::link_bones,
                    player::advance_players,
                    player::pose_bones
                )
                    .chain()
                    .in_set(AnimationSystems)
            );
    }
}
