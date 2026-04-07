//! A plugin for loading and saving scenes and configuration files in the WDE format.
use bevy::prelude::*;

mod utils;

#[doc(hidden)]
pub mod prelude {
    pub mod serialize {
        pub use crate::utils::serialize::*;
        pub use serde_json::*;
    }
}

pub struct ScenePlugin;
impl Plugin for ScenePlugin {
    fn build(&self, _app: &mut App) {}
}
