use bevy::{
    ecs::system::{SystemParamItem, lifetimeless::SRes},
    prelude::*
};
use wde_renderer::prelude::*;

use crate::render::{data::UiQuadsBinding, image::UiDrawBatches, pipeline::UiRenderPipeline};

/// Draws the UI on the swapchain, after the scene is resolved and before egui.
pub struct UiRenderPass;
impl RenderPass for UiRenderPass {
    type Params = ();

    fn describe(_params: &SystemParamItem<Self::Params>) -> RenderPassDesc {
        RenderPassDesc::default()
    }

    fn id() -> RenderPassId {
        500
    }
    fn label() -> &'static str {
        "ui"
    }
}

pub(crate) struct SubRenderPassUi;
impl RenderSubPass for SubRenderPassUi {
    type Params = (
        SRes<RenderAssets<UiRenderPipeline>>,
        SBinding<UiQuadsBinding>
    );

    fn describe((pipeline, ui_quads): &SystemParamItem<Self::Params>) -> RenderSubPassDesc {
        RenderSubPassDesc(vec![
            SubPassCommand::Pipeline(pipeline.iter().next().map(|(_, p)| p.0)),
            SubPassCommand::BindGroup(0, ui_quads.iter().next().map(|(_, b)| b.bind_group.clone())),
            SubPassCommand::Custom(draw_quads),
        ])
    }

    fn label() -> &'static str {
        "ui-quads"
    }
}

fn draw_quads<'pass>(world: &'pass World, render_pass: &mut RenderPassInstance<'pass>) {
    for (bind_group, quads) in &world.resource::<UiDrawBatches>().0 {
        render_pass.set_bind_group(1, bind_group);
        let _ = render_pass.draw(0..6, quads.clone());
    }
}
