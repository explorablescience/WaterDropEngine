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
            materials::TerrainMaterialsBinding, terrain_buffer::TerrainBufferBinding,
            terrain_mesh::TerrainRenderPassMesh
        },
        passes::pipeline::TerrainRenderPipeline,
        renderer_gpu::{TerrainChunkArrayBg, TerrainRendererGPU}
    }
};

pub(crate) struct SubRenderPassTerrainGround;
impl SubRenderPassTerrainGround {
    pub fn extract(
        mut main_world: ResMut<MainWorld>,
        mut render_terrain_extractor: ResMut<TerrainExtractor>,
        mut gpu_terrain_renderer: ResMut<TerrainRendererGPU>
    ) {
        let (heightmap_array, splatmap_array, pos_to_layer, dirty, chunk_count) = {
            let mut query = main_world.query::<&TerrainRenderer>();
            if let Ok(renderer) = query.single(&main_world) {
                (
                    renderer.heightmap_array.clone(),
                    renderer.splatmap_array.clone(),
                    renderer.pos_to_layer.clone(),
                    renderer.dirty.clone(),
                    renderer.pos_to_layer.len() as u32
                )
            } else {
                return;
            }
        };
        let mut main_terrain_extractor = main_world
            .get_resource_mut::<TerrainExtractor>()
            .expect("Main world is missing TerrainExtractor");
        TerrainRendererGPU::extract_dirty(
            &mut main_terrain_extractor,
            &mut render_terrain_extractor,
            heightmap_array,
            splatmap_array,
            pos_to_layer,
            dirty,
            chunk_count,
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
        SBinding<TerrainChunkArrayBg>
    );

    fn describe(
        (
            terrain_mesh,
            meshes,
            pipeline,
            camera,
            terrain_materials,
            terrain_buffer,
            terrain_renderer,
            chunk_array_bg
        ): &SystemParamItem<Self::Params>
    ) -> RenderSubPassDesc {
        let mesh = match meshes.get(terrain_mesh.deferred_mesh.as_ref().unwrap().id()) {
            Some(m) => m,
            None => return RenderSubPassDesc::default()
        };

        if !terrain_renderer.ready {
            return RenderSubPassDesc::default();
        }

        let render_bind_group = terrain_renderer
            .render_bind_group
            .as_ref()
            .and_then(|h| chunk_array_bg.get(h))
            .map(|bg| bg.bind_group.clone());

        // One instanced draw call for all chunks.
        let instance_count = terrain_renderer.chunk_count;
        let batch = DrawCommandsBatch {
            bind_group: None,
            index_range: 0..mesh.index_count,
            instance_range: 0..instance_count
        };

        RenderSubPassDesc(vec![
            SubPassCommand::Pipeline(Some(pipeline.iter().next().map(|(_, p)| p.0)).flatten()),
            SubPassCommand::Mesh(terrain_mesh.deferred_mesh.as_ref().map(|m| m.id())),
            SubPassCommand::BindGroup(0, camera.iter().next().map(|(_, c)| c.bind_group.clone())),
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
            SubPassCommand::BindGroup(3, render_bind_group),
            SubPassCommand::DrawBatches(vec![batch]),
        ])
    }

    fn label() -> &'static str {
        "terrain-ground"
    }
}
