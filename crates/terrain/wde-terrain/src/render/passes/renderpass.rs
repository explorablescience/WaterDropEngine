use wde_logger::prelude::*;
use bevy::prelude::*;
use wde_renderer::{passes::render_graph::DrawCommandsBatch, prelude::*};
use wde_camera::prelude::*;

use crate::{prelude::{TerrainExtractor, TerrainRenderer}, render::{dependencies::{materials::TerrainMaterialArrays, terrain_buffer::TerrainBuffer, terrain_mesh::TerrainRenderPassMesh}, passes::pipeline::GpuTerrainRenderPipeline, renderer_gpu::TerrainRendererGPU}};


#[derive(Resource, Default)]
pub(crate) struct TerrainRenderPass;
impl TerrainRenderPass {
    pub fn extract(
        main_terrain_extractor: ExtractWorld<Res<TerrainExtractor>>,
        mut render_terrain_extractor: ResMut<TerrainExtractor>,
        terrain_renderer_query: ExtractWorld<Query<&TerrainRenderer>>,
        mut gpu_terrain_renderer: ResMut<TerrainRendererGPU>
    ) {
        let _span = debug_span!("terrain_render_pass_extract").entered();

        // Extract the dirty terrain tiles
        TerrainRendererGPU::extract_dirty(
            &main_terrain_extractor,
            &mut render_terrain_extractor,
            *terrain_renderer_query,
            &mut gpu_terrain_renderer
        );
    }
}



impl RenderPassOld for TerrainRenderPass {
    fn render(&self, world: &mut World) {
        // Get the mesh
        let deferred_mesh = world.get_resource::<TerrainRenderPassMesh>().unwrap().deferred_mesh.as_ref().unwrap();
        let meshes = world.get_resource::<RenderAssets<GpuMesh>>().unwrap();
        let mesh = match meshes.get(deferred_mesh.id()) {
            Some(mesh) => mesh,
            None => return
        };

        // Create the batches of draw commands
        let mut batches = vec![];
        if let Some(terrain) = &world.get_resource::<TerrainRendererGPU>() && terrain.ready {
            for (i, tile) in terrain.tiles.iter().enumerate() {
                if let Some(bind_group) = &tile.render_bind_group {
                    batches.push(DrawCommandsBatch {
                        bind_group: Some((3, bind_group.clone())),
                        index_range: 0..mesh.index_count,
                        instance_range: i as u32..i as u32 + 1
                    });
                }
            }
        }

        // Create the pass and sub-pass descriptions
        let pass_desc = RenderPassDesc {
            attachments_depth: Some(RenderPassDescDepthAttachment {
                texture: Some(world.get_resource::<DepthTexture>().unwrap().texture.id()),
                ..default()
            }),
            ..default()
        };
        let sub_pass_desc = RenderSubPassDesc(vec![
            SubPassCommand::Pipeline(Some(world.get_resource::<RenderAssets<GpuTerrainRenderPipeline>>().unwrap().iter().next().map(|(_, p)| p.0)).flatten()),
            SubPassCommand::Mesh(world.get_resource::<TerrainRenderPassMesh>().unwrap().deferred_mesh.as_ref().map(|mesh| mesh.id())),
            SubPassCommand::BindGroup(0, world.get_resource::<CameraFeatureRender>().unwrap().bind_group.clone()),
            SubPassCommand::BindGroup(1, world.get_resource::<TerrainMaterialArrays>().unwrap().bind_group.clone()),
            SubPassCommand::BindGroup(2, world.get_resource::<TerrainBuffer>().unwrap().bind_group.clone()),
            SubPassCommand::DrawBatches(batches)
        ]);
        self.process(world, &pass_desc, &sub_pass_desc);
    }

    fn label(&self) -> &str {
        "terrain-ground"
    }
}
