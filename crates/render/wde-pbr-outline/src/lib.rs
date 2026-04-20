use bevy::prelude::*;
use wde_renderer::prelude::*;

mod extract;
mod material;
mod stencil_write;
mod transparent_outline;

use extract::{ExtractedOutlineInstances, extract_outline_instances};

pub use material::OutlineMaterial;

use crate::{stencil_write::StencilWritePlugin, transparent_outline::DrawOutlinePlugin};

pub mod prelude {
    pub use crate::OutlineMaterial;
}

pub struct PbrOutlinePlugin;
impl Plugin for PbrOutlinePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            DrawOutlinePlugin,
            StencilWritePlugin,
            RenderBindingRegisterPlugin::<OutlineMaterial>::default()
        ));

        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .init_resource::<ExtractedOutlineInstances>()
            .add_systems(Extract, extract_outline_instances);
    }
}
