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
            materials::TerrainMaterialArrays, terrain_buffer::TerrainBuffer,
            terrain_mesh::TerrainRenderPassMesh
        },
        passes::pipeline::GpuTerrainRenderPipeline,
        renderer_gpu::TerrainRendererGPU
    }
};

pub struct SubRenderPassTerrainGround;
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
        SRes<RenderAssets<GpuTerrainRenderPipeline>>,
        SBinding<CameraRender>,
        SRes<TerrainMaterialArrays>,
        SRes<TerrainBuffer>,
        SRes<TerrainRendererGPU>
    );

    fn describe(
        (
            terrain_render_pass_mesh,
            meshes,
            pipeline,
            camera,
            terrain_material_arrays,
            terrain_buffer,
            terrain_renderer
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
            SubPassCommand::BindGroup(1, terrain_material_arrays.bind_group.clone()),
            SubPassCommand::BindGroup(2, terrain_buffer.bind_group.clone()),
            SubPassCommand::DrawBatches(batches),
        ])
    }

    fn label() -> &'static str {
        "terrain-ground"
    }
}
