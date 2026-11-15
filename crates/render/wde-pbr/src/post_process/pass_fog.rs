use bevy::{
    ecs::system::{
        SystemParamItem,
        lifetimeless::{SRes, SResMut}
    },
    prelude::*
};
use wde_camera::prelude::CameraBinding;
use wde_renderer::prelude::*;

use super::data::FogBinding;
use crate::prelude::*;

#[cfg(feature = "atmosphere")]
const FOG_SHADER: &str = "core/render/post_process/fog_sky.frag.wgsl";
#[cfg(not(feature = "atmosphere"))]
const FOG_SHADER: &str = "core/render/post_process/fog.frag.wgsl";

/// Fog render pass: blends the fog over the opaque geometry and the sky of the MSAA [`RenderTexture`].
///
/// It runs after the atmosphere (ID 25) and before the transparent objects (ID 50), so that overlays are not fogged.
pub struct RenderPassFog;
impl RenderPass for RenderPassFog {
    type Params = SRenderData<RenderTexture>;

    fn describe(render_texture: &SystemParamItem<Self::Params>) -> RenderPassDesc {
        RenderPassDesc {
            attachments_colors: Some(vec![RenderPassDescColorAttachment {
                texture: render_texture
                    .iter()
                    .next()
                    .map(|(_, t)| t.get_texture(RenderTexture::BINDING).unwrap().id()),
                ..Default::default()
            }]),
            ..Default::default()
        }
    }

    fn id() -> RenderPassId {
        30
    }
    fn label() -> &'static str {
        "fog"
    }
}

#[derive(TypePath, Default, Clone, Debug)]
pub(crate) struct FogPipeline(pub CachedPipelineIndex);
impl RenderAsset for FogPipeline {
    type SourceAsset = RenderPipelineAsset<FogPipeline>;
    type Params = (
        SRes<AssetServer>,
        SResMut<PipelineManager>,
        SBinding<CameraBinding>,
        SBinding<FogBinding>,
        SBinding<LightsDataBinding>
    );

    fn prepare(
        _id: AssetId<Self::SourceAsset>,
        asset: Self::SourceAsset,
        (assets_server, pipeline_manager, camera, fog, lights): &mut SystemParamItem<Self::Params>
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        Ok(FogPipeline(pipeline_manager.create_render_pipeline(
            RenderPipelineDescriptor {
                label: "fog",
                vert: Some(assets_server.load("core/render/post_process/fullscreen.vert.wgsl")),
                frag: Some(assets_server.load(FOG_SHADER)),
                bind_group_layouts: vec![
                    camera.iter().next().map(|(_, c)| c.layout.clone()),
                    fog.iter().next().map(|(_, f)| f.layout.clone()),
                    lights.iter().next().map(|(_, l)| l.layout.clone()),
                ],
                depth: DepthDescriptor {
                    enabled: false,
                    ..default()
                },
                fragment_blend: Some(BlendState::ALPHA_BLENDING),
                render_targets: Some(vec![RenderTexture::FORMAT]),
                sample_count: MSAA_SAMPLE_COUNT,
                ..default()
            },
            asset
        )?))
    }
}

pub(crate) struct SubRenderPassFog;
impl RenderSubPass for SubRenderPassFog {
    type Params = (
        SRes<FogUniform>,
        SRes<RenderAssets<FogPipeline>>,
        SRes<PostProcessingMesh>,
        SBinding<CameraBinding>,
        SBinding<FogBinding>,
        SBinding<LightsDataBinding>
    );

    fn describe(
        (params, pipeline, mesh, camera, fog, lights): &SystemParamItem<Self::Params>
    ) -> RenderSubPassDesc {
        if params.limits[2] == 0.0 {
            return RenderSubPassDesc::default();
        }
        RenderSubPassDesc(vec![
            SubPassCommand::Pipeline(pipeline.iter().next().map(|(_, p)| p.0)),
            SubPassCommand::Mesh(mesh.0.as_ref().map(|h| h.id())),
            SubPassCommand::BindGroup(0, camera.iter().next().map(|(_, c)| c.bind_group.clone())),
            SubPassCommand::BindGroup(1, fog.iter().next().map(|(_, f)| f.bind_group.clone())),
            SubPassCommand::BindGroup(2, lights.iter().next().map(|(_, l)| l.bind_group.clone())),
            SubPassCommand::DrawBatches(vec![DrawCommandsBatch {
                index_range: 0..6,
                ..Default::default()
            }]),
        ])
    }

    fn label() -> &'static str {
        "fog"
    }
}
