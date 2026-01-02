use bevy::prelude::*;

use crate::{assets::GizmoMaterialPlugin, passes::GizmoFeaturesPlugin};

pub mod prelude {
    pub use crate::GizmosPlugin;
    pub use crate::assets::{cube_gizmo::CubeGizmoMesh, gizmo_material::{GizmoMaterial, GizmoMaterialAsset}};
}

pub mod assets;
pub mod passes;

pub struct GizmosPlugin;
impl Plugin for GizmosPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(GizmoMaterialPlugin)
            .add_plugins(GizmoFeaturesPlugin);
    }
}
