use bevy::prelude::*;

use crate::{assets::GizmoMaterialPlugin, passes::GizmoFeaturesPlugin};

pub mod assets;
pub mod passes;

pub struct GizmosPlugin;
impl Plugin for GizmosPlugin {
    fn build(&self, app: &mut App) {
        // Add the different plugins
        app
            .add_plugins(GizmoMaterialPlugin)
            .add_plugins(GizmoFeaturesPlugin);
    }
}
