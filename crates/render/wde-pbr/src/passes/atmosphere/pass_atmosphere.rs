use bevy::{
    ecs::system::{
        SystemParamItem,
        lifetimeless::{SRes, SResMut}
    },
    prelude::*
};
use wde_camera::prelude::CameraBinding;
use wde_renderer::prelude::*;

use crate::prelude::*;

/// Bind group exposing the resolved linear depth texture so the atmosphere shader
/// can test whether a pixel has geometry and discard it.
#[derive(TypePath, Default, Clone, Asset)]
pub struct AtmosphereDepthBinding;
impl RenderBinding for AtmosphereDepthBinding {
    type Params = SRenderData<DeferredTextures>;

    fn describe(
        &mut self,
        deferred_textures: &SystemParamItem<Self::Params>,
        builder: &mut RenderBindingBuilder
    ) {
        builder
            .add_texture_view(deferred_textures, DeferredTextures::DEPTH_RESOLVED_IDX)
            .add_texture_sampler(deferred_textures, DeferredTextures::DEPTH_RESOLVED_IDX);
    }

    fn has_dependencies(&self) -> bool {
        true
    }

    fn label(&self) -> &str {
        "atmosphere-depth-binding"
    }
}

/// Atmosphere render pass — renders a full-screen sky into the MSAA render texture.
///
/// It runs after deferred lighting (ID 20) and before transparent objects (ID 50).
pub struct RenderPassAtmosphere;
impl RenderPass for RenderPassAtmosphere {
    type Params = SRenderData<RenderTexture>;

    fn describe(render_texture: &SystemParamItem<Self::Params>) -> RenderPassDesc {
        RenderPassDesc {
            attachments_colors: Some(vec![RenderPassDescColorAttachment {
                texture: render_texture
                    .iter()
                    .next()
                    .map(|(_, t)| t.get_texture(RenderTexture::BINDING).unwrap().id()),
                load: LoadOp::Load,
                ..Default::default()
            }]),
            ..Default::default()
        }
    }

    fn id() -> RenderPassId {
        25
    }
    fn label() -> &'static str {
        "atmosphere"
    }
}

#[derive(TypePath, Default, Clone, Debug)]
pub struct AtmospherePipeline(pub CachedPipelineIndex);
impl RenderAsset for AtmospherePipeline {
    type SourceAsset = RenderPipelineAsset<AtmospherePipeline>;
    type Params = (
        SRes<AssetServer>,
        SResMut<PipelineManager>,
        SBinding<CameraBinding>,
        SBinding<AtmosphereDepthBinding>,
        SBinding<LightsDataBinding>
    );

    fn prepare(
        asset: Self::SourceAsset,
        (assets_server, pipeline_manager, camera, depth_binding, lights): &mut SystemParamItem<
            Self::Params
        >
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        Ok(AtmospherePipeline(
            pipeline_manager.create_render_pipeline(
                RenderPipelineDescriptor {
                    label: "atmosphere",
                    vert: Some(assets_server.load("core/render/atmosphere/atmosphere.vert.wgsl")),
                    frag: Some(assets_server.load("core/render/atmosphere/atmosphere.frag.wgsl")),
                    bind_group_layouts: vec![
                        camera.iter().next().map(|(_, c)| c.layout.clone()),
                        depth_binding.iter().next().map(|(_, d)| d.layout.clone()),
                        lights.iter().next().map(|(_, l)| l.layout.clone()),
                    ],
                    depth: DepthDescriptor {
                        enabled: false,
                        ..default()
                    },
                    sample_count: MSAA_SAMPLE_COUNT,
                    ..default()
                },
                asset
            )?
        ))
    }
}

pub struct SubRenderPassAtmosphere;
impl RenderSubPass for SubRenderPassAtmosphere {
    type Params = (
        SRes<RenderAssets<AtmospherePipeline>>,
        SRes<PostProcessingMesh>,
        SBinding<CameraBinding>,
        SBinding<AtmosphereDepthBinding>,
        SBinding<LightsDataBinding>
    );

    fn describe(
        (pipeline, mesh, camera, depth_binding, lights): &SystemParamItem<Self::Params>
    ) -> RenderSubPassDesc {
        RenderSubPassDesc(vec![
            SubPassCommand::Pipeline(Some(pipeline.iter().next().map(|(_, p)| p.0)).flatten()),
            SubPassCommand::Mesh(mesh.0.as_ref().map(|h| h.id())),
            SubPassCommand::BindGroup(0, camera.iter().next().map(|(_, c)| c.bind_group.clone())),
            SubPassCommand::BindGroup(
                1,
                depth_binding
                    .iter()
                    .next()
                    .map(|(_, d)| d.bind_group.clone())
            ),
            SubPassCommand::BindGroup(2, lights.iter().next().map(|(_, l)| l.bind_group.clone())),
            SubPassCommand::DrawBatches(vec![DrawCommandsBatch {
                index_range: 0..6,
                ..Default::default()
            }]),
        ])
    }

    fn label() -> &'static str {
        "atmosphere"
    }
}
