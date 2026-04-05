use bevy::prelude::*;

mod utils;

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
