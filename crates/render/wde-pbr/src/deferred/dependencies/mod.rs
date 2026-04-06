use bevy::prelude::{App, Plugin};
use wde_renderer::assets::MaterialsPluginRegister;

pub(crate) mod batches;
pub(crate) mod model;
pub(crate) mod pbr_material;
pub(crate) mod pbr_ssbo_transforms;

pub use model::PbrModel;
pub use pbr_material::PbrMaterial;
pub use pbr_ssbo_transforms::{SSBO_TRANSFORM_MAX_ENTITY, SsboTransform};

use crate::{
    deferred::dependencies::batches::BatchesPlugin,
    prelude::{model::PbrModelRegistryPlugin, pbr_ssbo_transforms::SsboTransformPlugin}
};

pub(crate) struct DeferredDependenciesPlugin;
impl Plugin for DeferredDependenciesPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            SsboTransformPlugin,
            BatchesPlugin,
            PbrModelRegistryPlugin,
            MaterialsPluginRegister::<PbrMaterial>::default()
        ));
    }
}
