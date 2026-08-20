use bevy::{
    ecs::system::{
        SystemParamItem,
        lifetimeless::{SRes, SResMut}
    },
    prelude::*
};
use wde_camera::prelude::*;
use wde_renderer::prelude::{Color, *};

use crate::{passes::shadow::ShadowSamplingBinding, prelude::*};

#[derive(TypePath, Default, Clone, Asset)]
pub(crate) struct RenderBindingResolved;
impl RenderBinding for RenderBindingResolved {
    type Params = SRenderData<DeferredTextures>;

    fn describe(
        &mut self,
        deferred_textures: &SystemParamItem<Self::Params>,
        builder: &mut RenderBindingBuilder
    ) {
        builder
            .add_texture_view(deferred_textures, DeferredTextures::DEPTH_RESOLVED_IDX)
            .add_texture_sampler(deferred_textures, DeferredTextures::DEPTH_RESOLVED_IDX)
            .add_texture_view(deferred_textures, DeferredTextures::ALBEDO_RESOLVED_IDX)
            .add_texture_sampler(deferred_textures, DeferredTextures::ALBEDO_RESOLVED_IDX)
            .add_texture_view(deferred_textures, DeferredTextures::NORMAL_RESOLVED_IDX)
            .add_texture_sampler(deferred_textures, DeferredTextures::NORMAL_RESOLVED_IDX)
            .add_texture_view(deferred_textures, DeferredTextures::AO_RESOLVED_IDX)
            .add_texture_sampler(deferred_textures, DeferredTextures::AO_RESOLVED_IDX);
    }

    fn has_dependencies(&self) -> bool {
        true
    }

    fn label(&self) -> &str {
        "deferred-textures-resolved-binding"
    }
}

#[derive(TypePath, Default, Clone, Asset)]
pub struct LightsDataBinding;
impl RenderBinding for LightsDataBinding {
    type Params = SRenderData<LightsData>;

    fn describe(
        &mut self,
        lights_data: &SystemParamItem<Self::Params>,
        builder: &mut RenderBindingBuilder
    ) {
        builder
            .add_buffer(lights_data, LightsData::LIGHTS_BUFFER_IDX);
        #[cfg(feature = "atmosphere")]
        builder
            .add_buffer(lights_data, LightsData::ATMOSPHERE_PARAMS_BUFFER_IDX);
        builder
            .add_buffer(lights_data, LightsData::PBR_PARAMS_BUFFER_IDX);
    }

    fn label(&self) -> &str {
        "lights-data-binding"
    }
}

/// The render pass for rendering the lighting of opaque objects in the deferred rendering pipeline.
/// It uses the G-buffer textures rendered in the previous render pass [`crate::prelude::RenderPassGBuffer`] to calculate the lighting, and writes to the final render texture, which is then presented on the screen.
///
/// Note:
///  - This render pass is not responsible for rendering transparent objects, which are rendered in a separate render pass[`crate::prelude::RenderPassTransparent`].
///  - This render pass clears the color attachment (MSAA render texture) to black.
///  - It has a render index of 20.
pub struct RenderPassDeferredLighting;
impl RenderPass for RenderPassDeferredLighting {
    type Params = SRenderData<RenderTexture>;

    fn describe(render_texture: &SystemParamItem<Self::Params>) -> RenderPassDesc {
        RenderPassDesc {
            attachments_colors: Some(vec![RenderPassDescColorAttachment {
                texture: render_texture
                    .iter()
                    .next()
                    .map(|(_, t)| t.get_texture(RenderTexture::BINDING).unwrap().id()),
                load: LoadOp::Clear(Color::BLACK),
                ..Default::default()
            }]),
            ..Default::default()
        }
    }

    fn id() -> RenderPassId {
        20
    }
    fn label() -> &'static str {
        "deferred-lighting"
    }
}

#[derive(TypePath, Default, Clone, Debug)]
pub(crate) struct DeferredLightingPipeline(pub CachedPipelineIndex);
impl RenderAsset for DeferredLightingPipeline {
    type SourceAsset = RenderPipelineAsset<DeferredLightingPipeline>;
    type Params = (
        SRes<AssetServer>,
        SResMut<PipelineManager>,
        SBinding<CameraBinding>,
        SBinding<RenderBindingResolved>,
        SBinding<LightsDataBinding>,
        SBinding<ShadowSamplingBinding>
    );

    fn prepare(
        asset: Self::SourceAsset,
        (assets_server, pipeline_manager, camera, render_binding, lights_buffer, shadow_binding): &mut SystemParamItem<
            Self::Params,
        >
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        Ok(DeferredLightingPipeline(
            pipeline_manager.create_render_pipeline(
                RenderPipelineDescriptor {
                    label: "deferred-lighting",
                    vert: Some(assets_server.load("core/render/pbr/lighting.vert.wgsl")),
                    frag: Some(assets_server.load("core/render/pbr/lighting.frag.wgsl")),
                    bind_group_layouts: vec![
                        camera.iter().next().map(|(_, c)| c.layout.clone()),
                        render_binding.iter().next().map(|(_, d)| d.layout.clone()),
                        lights_buffer.iter().next().map(|(_, b)| b.layout.clone()),
                        shadow_binding.iter().next().map(|(_, s)| s.layout.clone()),
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

pub(crate) struct SubRenderPassLightingPbr;
impl RenderSubPass for SubRenderPassLightingPbr {
    type Params = (
        SRes<RenderAssets<DeferredLightingPipeline>>,
        SRes<PostProcessingMesh>,
        SBinding<CameraBinding>,
        SBinding<RenderBindingResolved>,
        SBinding<LightsDataBinding>,
        SBinding<ShadowSamplingBinding>
    );

    fn describe(
        (pipeline, mesh, camera, rbinding, lights, shadow): &SystemParamItem<Self::Params>
    ) -> RenderSubPassDesc {
        RenderSubPassDesc(vec![
            SubPassCommand::Pipeline(Some(pipeline.iter().next().map(|(_, p)| p.0)).flatten()),
            SubPassCommand::Mesh(mesh.0.as_ref().map(|m| m.id())),
            SubPassCommand::BindGroup(0, camera.iter().next().map(|(_, c)| c.bind_group.clone())),
            SubPassCommand::BindGroup(1, rbinding.iter().next().map(|(_, d)| d.bind_group.clone())),
            SubPassCommand::BindGroup(2, lights.iter().next().map(|(_, b)| b.bind_group.clone())),
            SubPassCommand::BindGroup(3, shadow.iter().next().map(|(_, s)| s.bind_group.clone())),
            SubPassCommand::DrawBatches(vec![DrawCommandsBatch {
                index_range: 0..6,
                ..Default::default()
            }]),
        ])
    }

    fn label() -> &'static str {
        "deferred-lighting"
    }
}
