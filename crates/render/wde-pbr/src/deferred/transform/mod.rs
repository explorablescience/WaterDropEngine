use bevy::prelude::*;

pub(crate) mod ssbo_transforms;
mod registry;

pub use ssbo_transforms::{PbrSsboTransform, SSBO_TRANSFORM_MAX_ENTITY};
pub use registry::*;

use crate::{
    deferred::transform::{registry::SsboTransformRegistryPlugin, ssbo_transforms::SsboTransformPlugin}
};

pub(crate) struct PbrTransformPlugin;
impl Plugin for PbrTransformPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((SsboTransformPlugin, SsboTransformRegistryPlugin));
    }
}
