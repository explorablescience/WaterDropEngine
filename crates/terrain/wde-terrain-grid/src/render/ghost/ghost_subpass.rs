use wde_logger::prelude::*;

use bevy::{
    ecs::system::{SystemParamItem, lifetimeless::SRes},
    prelude::*
};
use wde_camera::prelude::*;
use wde_pbr::prelude::*;
use wde_renderer::prelude::*;

use crate::render::ghost::{ghost_material::GhostMaterial, ghost_pipeline::GhostRenderPipeline};

pub(crate) struct SubRenderPassGhost;
impl RenderSubPass for SubRenderPassGhost {
    type Params = (
        SRes<RenderAssets<GhostRenderPipeline>>,
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
        "ghost"
    }
}

#[allow(clippy::type_complexity)]
pub(crate) fn extract_entities(
    main_query: ExtractWorld<
        Query<(
            RenderEntity,
            &Mesh3d,
            &PbrMaterial3d<GhostMaterial>,
            &PbrSsboTransformUuid
        )>
    >,
    mut commands: Commands
) {
    for (entity, mesh, material, transform_uuid) in main_query.iter() {
        commands.get_entity(entity).unwrap().insert((
            mesh.clone(),
            material.clone(),
            transform_uuid.clone()
        ));
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct PushConstants {
    first_vertex: u32,
    first_index: u32,
    transform_index: u32
}

fn draw_custom<'pass>(world: &'pass World, render_pass: &mut RenderPassInstance<'pass>) {
    let meshes = world.get_resource::<RenderAssets<GpuMesh>>().unwrap();
    let materials = world
        .get_resource::<RenderAssets<GpuRenderBinding<GhostMaterial>>>()
        .unwrap();
    let ssbo_transform = world.get_resource::<PbrUuidRegistry>().unwrap();
    let mut query = match world.try_query::<(
        &Mesh3d,
        &PbrMaterial3d<GhostMaterial>,
        &PbrSsboTransformUuid
    )>() {
        Some(query) => query,
        None => return
    };

    for (mesh, material, transform_uuid) in query.iter(world) {
        // Set the mesh
        let mesh = match meshes.get(&mesh.0) {
            Some(mesh) => mesh,
            None => continue
        };

        // Set push constants
        render_pass.set_push_constants(
            ShaderStages::VERTEX,
            bytemuck::bytes_of(&[PushConstants {
                first_vertex: mesh.ssbo_first_vertex,
                first_index: mesh.ssbo_first_index,
                transform_index: ssbo_transform.get(&transform_uuid.0).unwrap_or(10000)
            }])
        );

        // Set the material
        let material = match materials.get(&material.0) {
            Some(material) => material,
            None => continue
        };
        render_pass.set_bind_group(3, &material.bind_group);

        // Draw the mesh
        if let Err(e) = render_pass.draw(0..mesh.index_count, 0..1) {
            error!("Failed to draw: {:?}.", e);
        }
    }
}
