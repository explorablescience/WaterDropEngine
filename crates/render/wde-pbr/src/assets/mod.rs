mod pbr_material;

pub use pbr_material::*;

use bevy::prelude::*;
use wde_renderer::prelude::*;

pub(crate) struct PbrAssetsPlugin;
impl Plugin for PbrAssetsPlugin {
    fn build(&self, app: &mut App) {
        // Register the extract commands of the material
        app.add_plugins(MaterialsPluginRegister::<PbrMaterial>::default());
    }
}
