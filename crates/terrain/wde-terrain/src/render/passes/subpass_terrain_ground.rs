use bevy::{
    ecs::system::{SystemParamItem, lifetimeless::SRes},
    prelude::*
};
use wde_camera::prelude::*;
use wde_renderer::prelude::*;

use crate::{
    prelude::{TerrainExtractor, TerrainRenderer},
    render::{
        dependencies::{
            materials::TerrainMaterialsBinding, terrain_buffer::{TerrainBufferBinding},
            terrain_mesh::TerrainRenderPassMesh
        },
        passes::pipeline::TerrainRenderPipeline,
        renderer_gpu::{TerrainRendererGPU, TerrainTileBgRender}
    }
};

pub(crate) struct SubRenderPassTerrainGround;
impl SubRenderPassTerrainGround {
    pub fn extract(
        main_terrain_extractor: ExtractWorld<Res<TerrainExtractor>>,
        mut render_terrain_extractor: ResMut<TerrainExtractor>,
        terrain_renderer_query: ExtractWorld<Query<&TerrainRenderer>>,
        mut gpu_terrain_renderer: ResMut<TerrainRendererGPU>
    ) {
        TerrainRendererGPU::extract_dirty(
            &main_terrain_extractor,
            &mut render_terrain_extractor,
            *terrain_renderer_query,
            &mut gpu_terrain_renderer
        );
    }
}
impl RenderSubPass for SubRenderPassTerrainGround {
    type Params = (
        SRes<TerrainRenderPassMesh>,
        SRes<RenderAssets<GpuMesh>>,
        SRes<RenderAssets<TerrainRenderPipeline>>,
        SBinding<CameraBinding>,
        SBinding<TerrainMaterialsBinding>,
        SBinding<TerrainBufferBinding>,
        SRes<TerrainRendererGPU>,
        SBinding<TerrainTileBgRender>
    );

    fn describe(
        (
            terrain_render_pass_mesh,
            meshes,
            pipeline,
            camera,
            terrain_materials,
            terrain_buffer,
            terrain_renderer,
            terrain_tile_bg_render
        ): &SystemParamItem<Self::Params>
    ) -> RenderSubPassDesc {
        // Create the batches of draw commands
        let mesh = match meshes.get(
            terrain_render_pass_mesh
                .deferred_mesh
                .as_ref()
                .unwrap()
                .id()
        ) {
            Some(mesh) => mesh,
            None => return RenderSubPassDesc::default()
        };
        let mut batches = vec![];
        if terrain_renderer.ready {
            for (i, tile) in terrain_renderer.tiles.iter().enumerate() {
                if let Some(bind_group) = &tile.render_bind_group {
                    let bind_group = match terrain_tile_bg_render.get(bind_group) {
                        Some(bg) => bg.bind_group.clone(),
                        None => continue
                    };
                    batches.push(DrawCommandsBatch {
                        bind_group: Some((3, bind_group.clone())),
                        index_range: 0..mesh.index_count,
                        instance_range: i as u32..i as u32 + 1
                    });
                }
            }
        }

        // Create the sub-pass description
        RenderSubPassDesc(vec![
            SubPassCommand::Pipeline(Some(pipeline.iter().next().map(|(_, p)| p.0)).flatten()),
            SubPassCommand::Mesh(
                terrain_render_pass_mesh
                    .deferred_mesh
                    .as_ref()
                    .map(|mesh| mesh.id())
            ),
            SubPassCommand::BindGroup(
                0,
                camera
                    .iter()
                    .next()
                    .map(|(_, camera)| camera.bind_group.clone())
            ),
            SubPassCommand::BindGroup(
                1,
                terrain_materials
                    .iter()
                    .next()
                    .map(|(_, m)| m.bind_group.clone())
            ),
            SubPassCommand::BindGroup(
                2,
                terrain_buffer
                    .iter()
                    .next()
                    .map(|(_, b)| b.bind_group.clone())
            ),
            SubPassCommand::DrawBatches(batches),
        ])
    }

    fn label() -> &'static str {
        "terrain-ground"
    }
}
