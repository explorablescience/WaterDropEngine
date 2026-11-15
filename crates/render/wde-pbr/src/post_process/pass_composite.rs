use bevy::{
    ecs::system::{
        SystemParamItem,
        lifetimeless::{SRes, SResMut}
    },
    prelude::*
};
use wde_renderer::prelude::*;

use super::data::PostProcessCompositeBinding;

/// Final post-processing pass, rendering to the swapchain.
///
/// Its composite sub-pass applies, in a single full-screen draw, the chromatic aberration, the bloom,
/// the color grading, the tonemapping, the vignette, the film grain and the dithering.
/// Other full-screen effects can be added as sub-passes after it: they draw over the final image.
///
/// It has a render index of 100: passes rendering to the swapchain over the scene (such as the UI) come after it.
pub struct RenderPassPostProcess;
impl RenderPass for RenderPassPostProcess {
    type Params = ();

    fn describe(_: &SystemParamItem<Self::Params>) -> RenderPassDesc {
        RenderPassDesc::default()
    }
    fn id() -> RenderPassId {
        100
    }
    fn label() -> &'static str {
        "post-process"
    }
}

#[derive(TypePath, Default, Clone, Debug)]
pub(crate) struct CompositePipeline(pub CachedPipelineIndex);
impl RenderAsset for CompositePipeline {
    type SourceAsset = RenderPipelineAsset<CompositePipeline>;
    type Params = (
        SRes<AssetServer>,
        SResMut<PipelineManager>,
        SBinding<PostProcessCompositeBinding>
    );

    fn prepare(
        _id: AssetId<Self::SourceAsset>,
        asset: Self::SourceAsset,
        (assets_server, pipeline_manager, binding): &mut SystemParamItem<Self::Params>
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        Ok(CompositePipeline(pipeline_manager.create_render_pipeline(
            RenderPipelineDescriptor {
                label: "post-process-composite",
                vert: Some(assets_server.load("core/render/post_process/fullscreen.vert.wgsl")),
                frag: Some(assets_server.load("core/render/post_process/composite.frag.wgsl")),
                bind_group_layouts: vec![binding.iter().next().map(|(_, b)| b.layout.clone())],
                depth: DepthDescriptor {
                    enabled: false,
                    ..default()
                },
                ..default()
            },
            asset
        )?))
    }
}

pub(crate) struct SubRenderPassComposite;
impl RenderSubPass for SubRenderPassComposite {
    type Params = (
        SRes<RenderAssets<CompositePipeline>>,
        SRes<PostProcessingMesh>,
        SBinding<PostProcessCompositeBinding>
    );

    fn describe((pipeline, mesh, binding): &SystemParamItem<Self::Params>) -> RenderSubPassDesc {
        RenderSubPassDesc(vec![
            SubPassCommand::Pipeline(pipeline.iter().next().map(|(_, p)| p.0)),
            SubPassCommand::Mesh(mesh.0.as_ref().map(|h| h.id())),
            SubPassCommand::BindGroup(0, binding.iter().next().map(|(_, b)| b.bind_group.clone())),
            SubPassCommand::DrawBatches(vec![DrawCommandsBatch {
                index_range: 0..6,
                ..Default::default()
            }]),
        ])
    }

    fn label() -> &'static str {
        "post-process-composite"
    }
}
