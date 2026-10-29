use bevy::{
    ecs::system::{SystemParamItem, lifetimeless::SRes},
    prelude::*
};
use wde_camera::prelude::*;
use wde_pbr::prelude::*;
use wde_renderer::prelude::*;

use crate::{
    extract::{ExtractedOutlineInstance, ExtractedOutlineInstances},
    material::OutlineMaterial,
    stencil_write::pipeline::StencilWriteRenderPipeline,
    transparent_outline::pipeline::{OutlineRenderPipeline, PushConstants}
};

pub(crate) struct SubRenderPassOutline;
impl RenderSubPass for SubRenderPassOutline {
    type Params = (
        SRes<RenderAssets<OutlineRenderPipeline>>,
        SBinding<SsboMeshBinding>,
        SBinding<CameraBinding>,
        SBinding<SsboTransformBinding>
    );

    fn describe(
        (pipeline, ssbo_mesh, camera, transforms): &SystemParamItem<Self::Params>
    ) -> RenderSubPassDesc {
        RenderSubPassDesc(vec![
            SubPassCommand::Pipeline(Some(pipeline.iter().next().map(|(_, p)| p.0)).flatten()),
            SubPassCommand::BindGroup(
                0,
                ssbo_mesh.iter().next().map(|(_, m)| m.bind_group.clone())
            ),
            SubPassCommand::BindGroup(1, camera.iter().next().map(|(_, c)| c.bind_group.clone())),
            SubPassCommand::BindGroup(
                2,
                transforms.iter().next().map(|(_, t)| t.bind_group.clone())
            ),
            SubPassCommand::Custom(draw_instances),
        ])
    }

    fn label() -> &'static str {
        "pbr-outline"
    }
}

/// Draws each material group in turn: its silhouette to the stencil, then its outline outside of
/// it. As each group has its own stencil value, a group nested in another (e.g. furniture in a
/// house) keeps its outline.
fn draw_instances<'pass>(world: &'pass World, render_pass: &mut RenderPassInstance<'pass>) {
    let pipeline_manager = world.get_resource::<PipelineManager>().unwrap();
    let render_pipeline =
        |id: Option<CachedPipelineIndex>| match id.map(|id| pipeline_manager.get_pipeline(id)) {
            Some(CachedPipelineStatus::OkRender(pipeline)) => Some(pipeline),
            _ => None
        };
    let (Some(stencil_pipeline), Some(outline_pipeline)) = (
        render_pipeline(
            world
                .get_resource::<RenderAssets<StencilWriteRenderPipeline>>()
                .unwrap()
                .iter()
                .next()
                .map(|(_, p)| p.0)
        ),
        render_pipeline(
            world
                .get_resource::<RenderAssets<OutlineRenderPipeline>>()
                .unwrap()
                .iter()
                .next()
                .map(|(_, p)| p.0)
        )
    ) else {
        return;
    };
    let materials = world
        .get_resource::<RenderAssets<GpuRenderBinding<OutlineMaterial>>>()
        .unwrap();
    let extracted = world.get_resource::<ExtractedOutlineInstances>().unwrap();

    // Instances are sorted by material
    for (index, group) in extracted
        .0
        .chunk_by(|a, b| a.material == b.material)
        .enumerate()
    {
        let Some(material) = materials.get(group[0].material) else {
            continue;
        };
        // 0 is the stencil of non-outlined pixels
        render_pass.set_stencil_reference(index as u32 % 255 + 1);
        if render_pass.set_pipeline(stencil_pipeline).is_err() {
            return;
        }
        draw_group(world, render_pass, group);
        if render_pass.set_pipeline(outline_pipeline).is_err() {
            return;
        }
        render_pass.set_bind_group(3, &material.bind_group);
        draw_group(world, render_pass, group);
    }
}

fn draw_group(
    world: &World,
    render_pass: &mut RenderPassInstance,
    group: &[ExtractedOutlineInstance]
) {
    let meshes = world.get_resource::<RenderAssets<GpuMesh>>().unwrap();
    let transform_registry = world.get_resource::<PbrUuidRegistry>().unwrap();

    for instance in group {
        let (Some(mesh), Some(transform_id)) = (
            meshes.get(instance.mesh),
            transform_registry.get(&instance.transform_uuid)
        ) else {
            continue;
        };
        render_pass.set_push_constants(
            ShaderStages::VERTEX,
            bytemuck::bytes_of(&PushConstants {
                first_vertex: mesh.ssbo_first_vertex,
                first_index: mesh.ssbo_first_index,
                transform_id
            })
        );
        let _ = render_pass.draw(0..mesh.index_count, transform_id..(transform_id + 1));
    }
}
