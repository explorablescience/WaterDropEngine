use bevy::prelude::*;

pub(crate) mod ssbo_transforms;

pub use ssbo_transforms::{PbrSsboTransform, SSBO_TRANSFORM_MAX_ENTITY};

use crate::{
    deferred::transform::ssbo_transforms::SsboTransformPlugin,
    prelude::model::PbrModelRegistryPlugin
};

pub(crate) struct PbrTransformPlugin;
impl Plugin for PbrTransformPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((SsboTransformPlugin, PbrModelRegistryPlugin));
    }
}
