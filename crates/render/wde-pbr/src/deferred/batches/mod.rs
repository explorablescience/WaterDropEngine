use bevy::prelude::{App, Plugin};
use wde_renderer::assets::MaterialsPluginRegister;

pub(crate) mod build_batches;
pub(crate) mod model;
pub(crate) mod pbr_material;
pub(crate) mod ssbo_batches;

pub use model::PbrModel;
pub use pbr_material::PbrMaterial;

use crate::{
    deferred::batches::build_batches::BatchesPlugin,
    prelude::ssbo_batches::SsboInstancesToTransformPlugin
};

pub(crate) struct DeferredDependenciesPlugin;
impl Plugin for DeferredDependenciesPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            BatchesPlugin,
            MaterialsPluginRegister::<PbrMaterial>::default(),
            SsboInstancesToTransformPlugin
        ));
    }
}
