mod ghost_material;
mod ghost_pipeline;
mod ghost_subpass;

pub use ghost_material::*;

use bevy::prelude::*;
use wde_renderer::prelude::*;

pub(crate) struct GhostPlugin;
impl Plugin for GhostPlugin {
    fn build(&self, app: &mut App) {
        // Register the material
        app.add_plugins(MaterialsPluginRegister::<GhostMaterialAsset>::default());
    }
}
