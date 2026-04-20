use bevy::prelude::*;
use wde_pbr::prelude::*;
use wde_renderer::prelude::*;

mod pipeline;
mod subpass;

use pipeline::OutlineRenderPipeline;
use subpass::SubRenderPassOutline;

pub struct DrawOutlinePlugin;
impl Plugin for DrawOutlinePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RenderPipelineRegisterPlugin::<OutlineRenderPipeline>::default());
    }

    fn finish(&self, app: &mut App) {
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .world_mut()
            .get_resource_mut::<RenderGraph>()
            .unwrap()
            .add_sub_pass::<SubRenderPassOutline, RenderPassTransparent>();
    }
}
