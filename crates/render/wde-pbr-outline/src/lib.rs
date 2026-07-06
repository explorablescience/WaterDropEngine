use bevy::prelude::*;
use wde_renderer::prelude::*;

mod extract;
mod material;
mod selection_area;
mod stencil_write;
mod transparent_outline;

use extract::{ExtractedOutlineInstances, extract_outline_instances};
use selection_area::extract::{ExtractedSelectionAreaInstances, extract_selection_area_instances};

pub use material::OutlineMaterial;
pub use selection_area::SelectionAreaMaterial;

use crate::{
    selection_area::DrawSelectionAreaPlugin, stencil_write::StencilWritePlugin,
    transparent_outline::DrawOutlinePlugin
};

pub mod prelude {
    pub use crate::OutlineMaterial;
    pub use crate::SelectionAreaMaterial;
}

pub struct PbrOutlinePlugin;
impl Plugin for PbrOutlinePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            DrawOutlinePlugin,
            DrawSelectionAreaPlugin,
            StencilWritePlugin,
            RenderBindingRegisterPlugin::<OutlineMaterial>::default(),
            RenderBindingRegisterPlugin::<SelectionAreaMaterial>::default()
        ));

        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .init_resource::<ExtractedOutlineInstances>()
            .init_resource::<ExtractedSelectionAreaInstances>()
            .add_systems(
                Extract,
                (extract_outline_instances, extract_selection_area_instances)
            );
    }
}
