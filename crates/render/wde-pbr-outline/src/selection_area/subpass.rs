use bevy::{
    ecs::system::{SystemParamItem, lifetimeless::SRes},
    prelude::*
};
use wde_camera::prelude::*;
use wde_pbr::prelude::*;
use wde_renderer::prelude::*;

use crate::selection_area::{
    extract::ExtractedSelectionAreaInstances,
    material::SelectionAreaMaterial,
    pipeline::{PushConstants, SelectionAreaRenderPipeline}
};

pub(crate) struct SubRenderPassSelectionArea;
impl RenderSubPass for SubRenderPassSelectionArea {
    type Params = (
        SRes<RenderAssets<SelectionAreaRenderPipeline>>,
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
        "pbr-selection-area"
    }
}

fn draw_instances<'pass>(world: &'pass World, render_pass: &mut RenderPassInstance<'pass>) {
    let meshes = world.get_resource::<RenderAssets<GpuMesh>>().unwrap();
    let materials = world
        .get_resource::<RenderAssets<GpuRenderBinding<SelectionAreaMaterial>>>()
        .unwrap();
    let extracted = world
        .get_resource::<ExtractedSelectionAreaInstances>()
        .unwrap();
    let transform_registry = world.get_resource::<PbrUuidRegistry>().unwrap();

    for instance in extracted.0.iter() {
        let mesh = meshes.get(instance.mesh);
        let material = materials.get(instance.material);
        let transform_id = transform_registry.get(&instance.transform_uuid);
        let (Some(mesh), Some(material), Some(transform_id)) = (mesh, material, transform_id)
        else {
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
        render_pass.set_bind_group(3, &material.bind_group);
        let _ = render_pass.draw(0..mesh.index_count, transform_id..(transform_id + 1));
    }
}
