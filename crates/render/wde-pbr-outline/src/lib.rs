use bevy::prelude::*;
use wde_renderer::prelude::*;

mod extract;
mod material;
mod pass;
mod pipeline;
mod subpass;

use extract::{ExtractedOutlineInstances, extract_outline_instances};
use pass::RenderPassOutline;
use pipeline::OutlineRenderPipeline;
use subpass::SubRenderPassOutline;

pub use material::OutlineMaterial;

pub mod prelude {
    pub use crate::{OutlineMarker, OutlineMaterial, PbrOutlinePlugin};
}

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub struct OutlineMarker;

pub struct PbrOutlinePlugin;
impl Plugin for PbrOutlinePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<OutlineMarker>();

        app.add_plugins((
            RenderBindingRegisterPlugin::<OutlineMaterial>::default(),
            RenderPipelineRegisterPlugin::<OutlineRenderPipeline>::default()
        ));

        let render_app = app.get_sub_app_mut(RenderApp).unwrap();
        render_app
            .init_resource::<ExtractedOutlineInstances>()
            .add_systems(Extract, extract_outline_instances);
        render_app
            .world_mut()
            .get_resource_mut::<RenderGraph>()
            .unwrap()
            .add_pass::<RenderPassOutline>()
            .add_sub_pass::<SubRenderPassOutline, RenderPassOutline>();
    }
}
