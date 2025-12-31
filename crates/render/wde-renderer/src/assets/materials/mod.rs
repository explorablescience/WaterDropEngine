mod pbr_material;
mod gizmo_material;

pub use pbr_material::*;
pub use gizmo_material::*;

use bevy::prelude::*;

use super::MaterialsPluginRegister;

pub struct MaterialsPlugin;
impl Plugin for MaterialsPlugin {
    fn build(&self, app: &mut App) {
        // Register the extract commands of the materials
        app
            .add_plugins(MaterialsPluginRegister::<PbrMaterialAsset>::default())
            .add_plugins(MaterialsPluginRegister::<GizmoMaterialAsset>::default());

        // Register the components to the reflect system
        app
            .register_type::<PbrMaterial>()
            .register_type::<GizmoMaterial>();
    }
}
