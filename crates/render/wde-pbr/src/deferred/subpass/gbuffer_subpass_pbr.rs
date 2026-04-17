use crate::{
    deferred::subpass::{GBufferRenderPipeline, gbuffer_bindgroup::GBufferBindGroup},
    prelude::{BatchList, PbrMaterial}
};
use wde_logger::prelude::*;

use bevy::{
    ecs::system::{SystemParamItem, lifetimeless::SRes},
    prelude::*
};
use wde_camera::prelude::*;
use wde_renderer::prelude::*;

pub(crate) struct SubRenderPassGbufferPbr;
impl RenderSubPass for SubRenderPassGbufferPbr {
    type Params = (
        SRes<RenderAssets<GBufferRenderPipeline>>,
        SBinding<CameraBinding>,
        SBinding<GBufferBindGroup>,
        SBinding<SsboMeshBinding>
    );

    fn describe(
        (render_pipeline, camera, gbuffer_bg, ssbo_mesh): &SystemParamItem<Self::Params>
    ) -> RenderSubPassDesc {
        RenderSubPassDesc(vec![
            SubPassCommand::Pipeline(
                Some(render_pipeline.iter().next().map(|(_, p)| p.0)).flatten()
            ),
            SubPassCommand::BindGroup(
                0,
                ssbo_mesh.iter().next().map(|(_, m)| m.bind_group.clone())
            ),
            SubPassCommand::BindGroup(1, camera.iter().next().map(|(_, c)| c.bind_group.clone())),
            SubPassCommand::BindGroup(
                2,
                gbuffer_bg.iter().next().map(|(_, t)| t.bind_group.clone())
            ),
            SubPassCommand::Custom(draw_custom),
        ])
    }

    fn label() -> &'static str {
        "pbr-gbuffer-main"
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct PushConstants {
    first_vertex: u32,
    first_index: u32
}

fn draw_custom<'pass>(world: &'pass World, render_pass: &mut RenderPassInstance<'pass>) {
    let batches = world.get_resource::<BatchList>().unwrap();
    let materials = world
        .get_resource::<RenderAssets<GpuRenderBinding<PbrMaterial>>>()
        .unwrap();
    let meshes = world.get_resource::<RenderAssets<GpuMesh>>().unwrap();

    // Create the batches of draw commands
    let mut current_mesh_id = None;
    let mut current_index_count = 0;
    let mut current_material_id = None;
    for batch_key in batches.sorted_batches.iter() {
        let batch = match batches.batches.get(batch_key) {
            Some(batch) => batch,
            None => continue
        };
        // Set the mesh
        if current_mesh_id != Some(batch_key.mesh) {
            let mesh = match meshes.get(batch_key.mesh) {
                Some(mesh) => mesh,
                None => continue
            };
            current_index_count = mesh.index_count;
            current_mesh_id = Some(batch_key.mesh);

            // Set push constants
            render_pass.set_push_constants(
                ShaderStages::VERTEX,
                bytemuck::bytes_of(&[PushConstants {
                    first_vertex: mesh.ssbo_first_vertex,
                    first_index: mesh.ssbo_first_index
                }])
            );
        }

        // Set the material
        if current_material_id != Some(batch_key.material) {
            let material = match materials.get(batch_key.material) {
                Some(material) => material,
                None => continue
            };

            // Set the material bind group
            render_pass.set_bind_group(3, &material.bind_group);
            current_material_id = Some(batch_key.material);
        }

        // Draw the mesh
        if let Err(e) = render_pass.draw(
            0..current_index_count,
            batch.instances_offset
                ..(batch.instances_offset + batch.transform_ssbo_ids.len() as u32)
        ) {
            error!("Failed to draw: {:?}.", e);
        }
    }
}
