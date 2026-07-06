use bevy::{ecs::system::{SystemParamItem, lifetimeless::SRes}, prelude::*};
use wde_camera::prelude::*;
use wde_renderer::prelude::*;

use crate::{data::GizmoLinesBinding, extract::GizmoDrawCount, pipeline::GizmoRenderPipeline};

pub(crate) struct SubRenderPassGizmos;
impl RenderSubPass for SubRenderPassGizmos {
    type Params = (
        SRes<RenderAssets<GizmoRenderPipeline>>,
        SBinding<CameraBinding>,
        SBinding<GizmoLinesBinding>
    );

    fn describe(
        (pipeline, camera, gizmo_lines): &SystemParamItem<Self::Params>
    ) -> RenderSubPassDesc {
        RenderSubPassDesc(vec![
            SubPassCommand::Pipeline(pipeline.iter().next().map(|(_, p)| p.0)),
            SubPassCommand::BindGroup(0, camera.iter().next().map(|(_, c)| c.bind_group.clone())),
            SubPassCommand::BindGroup(
                1,
                gizmo_lines.iter().next().map(|(_, g)| g.bind_group.clone())
            ),
            SubPassCommand::Custom(draw_lines),
        ])
    }

    fn label() -> &'static str {
        "gizmos"
    }
}

fn draw_lines<'pass>(world: &'pass World, render_pass: &mut RenderPassInstance<'pass>) {
    let draw_count = world.get_resource::<GizmoDrawCount>().unwrap();
    if draw_count.0 == 0 {
        return;
    }

    let _ = render_pass.draw(0..draw_count.0, 0..1);
}
