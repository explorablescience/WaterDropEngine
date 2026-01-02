use bevy::prelude::*;
use wde_renderer::assets::MaterialsPluginRegister;

use crate::assets::gizmo_material::{GizmoMaterial, GizmoMaterialAsset};

pub mod cube_gizmo;
pub mod gizmo_material;

pub(crate) struct GizmoMaterialPlugin;
impl Plugin for GizmoMaterialPlugin {
    fn build(&self, app: &mut App) {
        // Register the gizmo material
        app.add_plugins(MaterialsPluginRegister::<GizmoMaterialAsset>::default());
        app.register_type::<GizmoMaterial>();
    }
}
