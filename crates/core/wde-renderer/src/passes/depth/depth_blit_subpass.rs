use bevy::{ecs::system::{SystemParamItem, lifetimeless::SRes}, prelude::*};
use crate::{passes::{depth::{depth_blit_pipeline::GpuDepthBlitRenderPipeline, depth_texture_msaa::DepthMSAATextureLayout}, post_process_mesh::PostProcessingMesh}, prelude::*};

pub struct SubRenderPassDepthBlit;
impl RenderSubPass for SubRenderPassDepthBlit {
    type Params = (SRes<RenderAssets<GpuDepthBlitRenderPipeline>>, SRes<PostProcessingMesh>, SRes<DepthMSAATextureLayout>);

    fn describe(
        (pipeline, mesh, depth_msaa_layout): &SystemParamItem<Self::Params>
    ) -> RenderSubPassDesc {
        RenderSubPassDesc(vec![
            SubPassCommand::Pipeline(Some(pipeline.iter().next().map(|(_, p)| p.0)).flatten()),
            SubPassCommand::Mesh(mesh.0.as_ref().map(|h| h.id())),
            SubPassCommand::BindGroup(0, depth_msaa_layout.bind_group.clone()),
            SubPassCommand::DrawBatches(vec![DrawCommandsBatch {
                index_range: 0..6,
                ..Default::default()
            }])
        ])
    }

    fn label() -> &'static str { "depth-blit-main" }
}
