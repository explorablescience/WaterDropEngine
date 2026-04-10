use bevy::prelude::{App, Plugin};

pub(crate) mod build_batches;
pub(crate) mod ssbo_batches;

use crate::{
    deferred::batches::build_batches::BatchesPlugin,
    prelude::ssbo_batches::SsboInstancesToTransformPlugin
};

pub(crate) struct DeferredDependenciesPlugin;
impl Plugin for DeferredDependenciesPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            BatchesPlugin,
            SsboInstancesToTransformPlugin
        ));
    }
}
