use bevy::prelude::*;
use wde_renderer::prelude::*;

mod build_batches;
mod ssbo_batches;

pub use build_batches::*;
pub use ssbo_batches::*;

use crate::deferred::batches::build_batches::BatchesPlugin;

pub(crate) struct DeferredDependenciesPlugin;
impl Plugin for DeferredDependenciesPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((BatchesPlugin, SsboInstancesToTransformPlugin));

        app.get_sub_app_mut(RenderApp).unwrap().add_systems(
            Render,
            (
                build_batches::build_batches,
                ssbo_batches::set_instances_to_transform
            )
                .chain()
                .in_set(RenderSet::Prepare)
        );
    }
}
