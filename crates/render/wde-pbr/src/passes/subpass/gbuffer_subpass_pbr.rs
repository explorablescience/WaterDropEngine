use wde_logger::prelude::*;
use bevy::{ecs::system::{SystemParamItem, lifetimeless::SRes}, prelude::*};
use wde_camera::render::CameraFeatureRender;
use wde_renderer::{prelude::*, ssbos::ssbo_mesh::SsboMesh};

use crate::{assets::PbrMaterialAsset, logic::{batches::Batches, ssbo::PbrSsbo}, passes::subpass::gbuffer_pipeline::GpuPbrGBufferRenderPipeline};


pub(crate) struct SubRenderPassGbufferPbr;
impl RenderSubPass for SubRenderPassGbufferPbr {
    type Params = (SRes<RenderAssets<GpuPbrGBufferRenderPipeline>>, SRes<SsboMesh>, SRes<CameraFeatureRender>, SRes<PbrSsbo>);

    fn describe(
        (render_pipeline, ssbo_mesh, camera_feature, pbr_ssbo): &SystemParamItem<Self::Params>
    ) -> RenderSubPassDesc {
        RenderSubPassDesc(vec![
            SubPassCommand::Pipeline(Some(render_pipeline.iter().next().map(|(_, p)| p.0)).flatten()),
            SubPassCommand::BindGroup(0, ssbo_mesh.bind_group.clone()),
            SubPassCommand::BindGroup(1, camera_feature.bind_group.clone()),
            SubPassCommand::BindGroup(2, pbr_ssbo.bind_group.clone()),
            SubPassCommand::Custom(draw_custom)
        ])
    }

    fn label() -> &'static str { "pbr-gbuffer-main" }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct PushConstants {
    first_vertex: u32,
    first_index: u32
}

fn draw_custom<'pass>(world: &'pass World, render_pass: &mut RenderPassInstance<'pass>) {
    let batches = world.get_resource::<Batches>().unwrap();
    let materials = world.get_resource::<RenderAssets<GpuMaterial<PbrMaterialAsset>>>().unwrap();
    let meshes = world.get_resource::<RenderAssets<GpuMesh>>().unwrap();

    // Create the batches of draw commands
    let mut current_mesh_id = None;
    let mut current_index_count = 0;
    let mut current_material_id = None;
    for batch in &batches.render_batches {
        // Set the mesh
        if current_mesh_id != Some(batch.mesh_id) {
            let mesh = match meshes.get(batch.mesh_id) {
                Some(mesh) => mesh,
                None => continue
            };
            current_index_count = mesh.index_count;
            current_mesh_id = Some(batch.mesh_id);

            // Set push constants
            render_pass.set_push_constants(ShaderStages::VERTEX, bytemuck::bytes_of(&[
                PushConstants {
                    first_vertex: mesh.first_vertex,
                    first_index: mesh.first_index
                }
            ]));
        }

        // Set the material
        if current_material_id != Some(batch.material_id) {
            let material = match materials.get(batch.material_id) {
                Some(material) => material,
                None => continue
            };

            // Set the material bind group
            render_pass.set_bind_group(3, &material.bind_group);
            current_material_id = Some(batch.material_id);
        }

        // Draw the mesh
        if let Err(e) = render_pass.draw(
            0..current_index_count,
            batch.first_instance..(batch.first_instance + batch.instance_count)
        ) {
            error!("Failed to draw: {:?}.", e);
        }
    }
}
