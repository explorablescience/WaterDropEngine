use bevy::prelude::{App, Plugin};
use wde_renderer::assets::MaterialsPluginRegister;

mod batches;
mod model;
mod pbr_material;
mod ssbo;

pub use batches::*;
pub use model::*;
pub use pbr_material::*;
pub use ssbo::*;

pub(crate) struct PbrDependenciesPlugin;
impl Plugin for PbrDependenciesPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PbrModelRegistryPlugin);
        app
            .add_plugins(PbrSsboPlugin)
            .add_plugins(BatchesPlugin);
        app.add_plugins(MaterialsPluginRegister::<PbrMaterial>::default());
    }
}
