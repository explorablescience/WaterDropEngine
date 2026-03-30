use wde_logger::prelude::*;

use bevy::prelude::*;
use wde_camera::prelude::*;
use wde_renderer::{prelude::*, ssbos::ssbo_mesh::SsboMesh};

use crate::{assets::PbrMaterialAsset, logic::{batches::Batches, ssbo::PbrSsbo, textures::PbrDeferredTextures}, prelude::{DirtyTransforms, ModelUuidToTransformUuidRender, PbrModelRegistry}};

use super::{GpuPbrGBufferRenderPipeline};

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct PushConstants {
    first_vertex: u32,
    first_index: u32
}

#[derive(Resource, Default)]
pub(crate) struct PbrGBufferRenderPass;
impl PbrGBufferRenderPass {
    pub fn extract(
        pbr_model_registry: ExtractWorld<Res<PbrModelRegistry>>,
        mut model_uuid_to_transform_id: ResMut<ModelUuidToTransformUuidRender>,
        main_dirty_transforms: ExtractWorld<Res<DirtyTransforms>>,
        mut render_dirty_transforms: ResMut<DirtyTransforms>
    ) {
        model_uuid_to_transform_id.0 = pbr_model_registry.model_uuid_to_transform_id.clone();
        render_dirty_transforms.0 = main_dirty_transforms.0.clone();
    }
}
impl RenderPass for PbrGBufferRenderPass {
    fn render(&self, world: &mut World) {
        let Some(deferred_textures) = world.get_resource::<PbrDeferredTextures>() else { return };
        let pass_desc = RenderPassDesc {
            attachments_depth: Some(RenderPassDescDepthAttachment {
                texture: Some(world.get_resource::<DepthTextureMSAA>().unwrap().texture.id()),
                load: LoadOp::Clear(1.0),
                ..default()
            }),
            attachments_colors: Some(vec![
                RenderPassDescColorAttachment {
                    texture: deferred_textures.depth.id(),
                    resolve_target: Some(deferred_textures.depth_resolved.id()),
                    load: LoadOp::Clear(WgpuColor { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }),
                    ..default()
                },
                RenderPassDescColorAttachment {
                    texture: deferred_textures.albedo.id(),
                    resolve_target: Some(deferred_textures.albedo_resolved.id()),
                    load: LoadOp::Clear(WgpuColor { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }),
                    ..default()
                },
                RenderPassDescColorAttachment {
                    texture: deferred_textures.normal.id(),
                    resolve_target: Some(deferred_textures.normal_resolved.id()),
                    load: LoadOp::Clear(WgpuColor { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }),
                    ..default()
                }
            ])
        };
        let sub_pass_desc = SubPassDesc(vec![
            SubPassCommand::Pipeline(Some(world.get_resource::<RenderAssets<GpuPbrGBufferRenderPipeline>>().unwrap().iter().next().map(|(_, p)| p.0)).flatten()),
            SubPassCommand::BindGroup(0, world.get_resource::<SsboMesh>().unwrap().bind_group.clone()),
            SubPassCommand::BindGroup(1, world.get_resource::<CameraFeatureRender>().unwrap().bind_group.clone()),
            SubPassCommand::BindGroup(2, world.get_resource::<PbrSsbo>().unwrap().bind_group.clone()),
            SubPassCommand::Custom(draw_custom)
        ]);
        self.process(world, &pass_desc, &sub_pass_desc);
    }

    fn label(&self) -> &str {
        "pbr-gbuffer"
    }
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
