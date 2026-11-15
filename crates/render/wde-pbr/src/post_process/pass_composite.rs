use bevy::{
    ecs::system::{
        SystemParamItem,
        lifetimeless::{SRes, SResMut}
    },
    prelude::*
};
use wde_renderer::{prelude::*, wgpu_utils::BindGroup};

use super::data::{AntiAliasingBinding, PostProcessCompositeBinding, PostProcessTextures};
use super::settings::{AntiAliasing, PostProcessUniform};
use super::smaa::{SmaaBlendBinding, SmaaBlendPipeline};

/// Composite pass, rendering to the LDR texture of the [`PostProcessTextures`].
///
/// It applies, in a single full-screen draw, the chromatic aberration, the bloom, the color grading,
/// the tonemapping, the vignette, the film grain and the dithering. It has a render index of 95.
pub struct RenderPassComposite;
impl RenderPass for RenderPassComposite {
    type Params = SRenderData<PostProcessTextures>;

    fn describe(textures: &SystemParamItem<Self::Params>) -> RenderPassDesc {
        RenderPassDesc {
            attachments_colors: Some(vec![RenderPassDescColorAttachment {
                texture: textures
                    .iter()
                    .next()
                    .and_then(|(_, t)| t.get_texture(PostProcessTextures::LDR_IDX))
                    .map(|t| t.id()),
                ..Default::default()
            }]),
            ..Default::default()
        }
    }
    fn id() -> RenderPassId {
        95
    }
    fn label() -> &'static str {
        "post-process-composite"
    }
}

/// Final post-processing pass, rendering to the swapchain.
///
/// Its anti-aliasing sub-pass copies the composited image to the swapchain, smoothing its edges (with the FXAA,
/// or the SMAA neighborhood blending).
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
                render_targets: Some(vec![PostProcessTextures::LDR_FORMAT]),
                ..default()
            },
            asset
        )?))
    }
}

#[derive(TypePath, Default, Clone, Debug)]
pub(crate) struct AntiAliasingPipeline(pub CachedPipelineIndex);
impl RenderAsset for AntiAliasingPipeline {
    type SourceAsset = RenderPipelineAsset<AntiAliasingPipeline>;
    type Params = (
        SRes<AssetServer>,
        SResMut<PipelineManager>,
        SBinding<AntiAliasingBinding>
    );

    fn prepare(
        _id: AssetId<Self::SourceAsset>,
        asset: Self::SourceAsset,
        (assets_server, pipeline_manager, binding): &mut SystemParamItem<Self::Params>
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        Ok(AntiAliasingPipeline(
            pipeline_manager.create_render_pipeline(
                RenderPipelineDescriptor {
                    label: "post-process-anti-aliasing",
                    vert: Some(assets_server.load("core/render/post_process/fullscreen.vert.wgsl")),
                    frag: Some(assets_server.load("core/render/post_process/fxaa.frag.wgsl")),
                    bind_group_layouts: vec![binding.iter().next().map(|(_, b)| b.layout.clone())],
                    depth: DepthDescriptor {
                        enabled: false,
                        ..default()
                    },
                    ..default()
                },
                asset
            )?
        ))
    }
}

fn fullscreen_draw(
    pipeline: Option<CachedPipelineIndex>,
    mesh: &PostProcessingMesh,
    bind_group: Option<BindGroup>
) -> RenderSubPassDesc {
    RenderSubPassDesc(vec![
        SubPassCommand::Pipeline(pipeline),
        SubPassCommand::Mesh(mesh.0.as_ref().map(|h| h.id())),
        SubPassCommand::BindGroup(0, bind_group),
        SubPassCommand::DrawBatches(vec![DrawCommandsBatch {
            index_range: 0..6,
            ..Default::default()
        }]),
    ])
}

pub(crate) struct SubRenderPassComposite;
impl RenderSubPass for SubRenderPassComposite {
    type Params = (
        SRes<RenderAssets<CompositePipeline>>,
        SRes<PostProcessingMesh>,
        SBinding<PostProcessCompositeBinding>
    );

    fn describe((pipeline, mesh, binding): &SystemParamItem<Self::Params>) -> RenderSubPassDesc {
        fullscreen_draw(
            pipeline.iter().next().map(|(_, p)| p.0),
            mesh,
            binding.iter().next().map(|(_, b)| b.bind_group.clone())
        )
    }

    fn label() -> &'static str {
        "post-process-composite"
    }
}

pub(crate) struct SubRenderPassAntiAliasing;
impl RenderSubPass for SubRenderPassAntiAliasing {
    type Params = (
        SRes<PostProcessUniform>,
        SRes<PostProcessingMesh>,
        SRes<RenderAssets<AntiAliasingPipeline>>,
        SBinding<AntiAliasingBinding>,
        SRes<RenderAssets<SmaaBlendPipeline>>,
        SBinding<SmaaBlendBinding>
    );

    fn describe(
        (params, mesh, pipeline, binding, smaa_pipeline, smaa_binding): &SystemParamItem<
            Self::Params
        >
    ) -> RenderSubPassDesc {
        // Until the SMAA is ready, the FXAA shader copies the image as is
        if params.anti_aliasing() == AntiAliasing::Smaa
            && let (Some((_, p)), Some((_, b))) =
                (smaa_pipeline.iter().next(), smaa_binding.iter().next())
        {
            return fullscreen_draw(Some(p.0), mesh, Some(b.bind_group.clone()));
        }
        fullscreen_draw(
            pipeline.iter().next().map(|(_, p)| p.0),
            mesh,
            binding.iter().next().map(|(_, b)| b.bind_group.clone())
        )
    }

    fn label() -> &'static str {
        "post-process-anti-aliasing"
    }
}
