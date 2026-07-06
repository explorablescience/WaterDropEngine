use bevy::{ecs::system::{SystemParamItem, lifetimeless::SRes}, prelude::*};
use wde_camera::prelude::*;
use wde_renderer::prelude::*;

use crate::{
    data::{GizmoLinesBinding, GizmoQuadsBinding},
    extract::{GizmoDrawCount, GizmoQuadDrawCount},
    pipeline::{GizmoQuadRenderPipeline, GizmoRenderPipeline}
};

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

pub(crate) struct SubRenderPassGizmoQuads;
impl RenderSubPass for SubRenderPassGizmoQuads {
    type Params = (
        SRes<RenderAssets<GizmoQuadRenderPipeline>>,
        SBinding<CameraBinding>,
        SBinding<GizmoQuadsBinding>
    );

    fn describe(
        (pipeline, camera, gizmo_quads): &SystemParamItem<Self::Params>
    ) -> RenderSubPassDesc {
        RenderSubPassDesc(vec![
            SubPassCommand::Pipeline(pipeline.iter().next().map(|(_, p)| p.0)),
            SubPassCommand::BindGroup(0, camera.iter().next().map(|(_, c)| c.bind_group.clone())),
            SubPassCommand::BindGroup(
                1,
                gizmo_quads.iter().next().map(|(_, g)| g.bind_group.clone())
            ),
            SubPassCommand::Custom(draw_quads),
        ])
    }

    fn label() -> &'static str {
        "gizmos-quads"
    }
}

fn draw_quads<'pass>(world: &'pass World, render_pass: &mut RenderPassInstance<'pass>) {
    let draw_count = world.get_resource::<GizmoQuadDrawCount>().unwrap();
    if draw_count.0 == 0 {
        return;
    }

    let _ = render_pass.draw(0..draw_count.0, 0..1);
}
