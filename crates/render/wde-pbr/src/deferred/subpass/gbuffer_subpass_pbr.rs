use crate::{
    deferred::subpass::{GBufferRenderPipeline, gbuffer_bindgroup::SsboTransformBinding},
    prelude::{BatchList, CAMERA_VIEW, LodSettings, PbrMaterial, is_batch_blended}
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
        SBinding<SsboTransformBinding>,
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
    first_index: u32,
    debug: u32
}

fn draw_custom<'pass>(world: &'pass World, render_pass: &mut RenderPassInstance<'pass>) {
    let batches = world.get_resource::<BatchList>().unwrap();
    let materials = world
        .get_resource::<RenderAssets<GpuRenderBinding<PbrMaterial>>>()
        .unwrap();
    let meshes = world.get_resource::<RenderAssets<GpuMesh>>().unwrap();
    let lod_settings = world.get_resource::<LodSettings>();

    // Create the batches of draw commands
    let mut current_material_id = None;
    for batch_key in batches.sorted_batches.iter() {
        if is_batch_blended(materials, batch_key) {
            continue;
        }
        let (Some(batch), Some(mesh)) =
            (batches.batches.get(batch_key), meshes.get(batch_key.mesh))
        else {
            continue;
        };
        if batch.lods(CAMERA_VIEW).next().is_none() {
            continue;
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

            // If the material has a stencil value, set it in the stencil buffer
            if let Some(stencil_value) = material.asset.stencil_value {
                render_pass.set_stencil_reference(stencil_value);
            } else {
                // Otherwise, set the stencil reference to 0 to avoid affecting other draw calls that use the stencil buffer
                render_pass.set_stencil_reference(0);
            }
        }

        // Draw each level of detail of the mesh
        for (level, instances) in batch.lods(CAMERA_VIEW) {
            let Some(lod) = mesh.lods.get(level) else {
                continue;
            };
            render_pass.set_push_constants(
                ShaderStages::VERTEX,
                bytemuck::bytes_of(&[PushConstants {
                    first_vertex: mesh.ssbo_first_vertex,
                    first_index: lod.first_index,
                    debug: LodSettings::debug_flags(lod_settings, level, mesh.lods.len())
                }])
            );
            if let Err(e) = render_pass.draw(0..lod.index_count, instances) {
                error!("Failed to draw: {:?}.", e);
            }
        }
    }
}
