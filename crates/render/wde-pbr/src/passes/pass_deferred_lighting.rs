use bevy::{
    ecs::system::{
        SystemParamItem,
        lifetimeless::{SRes, SResMut}
    },
    prelude::*
};
use wde_camera::prelude::*;
use wde_renderer::prelude::{Color, *};

use crate::prelude::*;

/// The render pass for rendering the lighting of opaque objects in the deferred rendering pipeline.
/// It uses the G-buffer textures rendered in the previous render pass [`crate::prelude::RenderPassGBuffer`] to calculate the lighting, and writes to the final render texture, which is then presented on the screen.
///
/// Note:
///  - This render pass is not responsible for rendering transparent objects, which are rendered in a separate render pass[`crate::prelude::RenderPassTransparent`].
///  - This render pass clears the color attachment (MSAA render texture) to black.
///  - It has a render index of 20.
pub struct RenderPassDeferredLighting;
impl RenderPass for RenderPassDeferredLighting {
    type Params = SRes<RenderTexture>;

    fn describe(render_texture: &SystemParamItem<Self::Params>) -> RenderPassDesc {
        RenderPassDesc {
            attachments_colors: Some(vec![RenderPassDescColorAttachment {
                texture: render_texture.texture.id(),
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
        SBinding<CameraRender>
    );

    fn prepare(
        asset: Self::SourceAsset,
        (assets_server, pipeline_manager, camera): &mut SystemParamItem<Self::Params>
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        Ok(DeferredLightingPipeline(
            pipeline_manager.create_render_pipeline(
                RenderPipelineDescriptor {
                    label: "deferred-lighting",
                    vert: Some(assets_server.load("core/render/pbr/lighting_vert.wgsl")),
                    frag: Some(assets_server.load("core/render/pbr/lighting_frag.wgsl")),
                    bind_group_layouts: vec![
                        camera.iter().next().map(|(_, c)| c.layout.clone()),
                        Some(PbrDeferredTexturesLayout::layout()),
                        Some(LightsFeatureBuffer::layout()),
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
        SBinding<CameraRender>,
        SRes<PbrDeferredTexturesLayout>,
        SRes<LightsFeatureBuffer>
    );

    fn describe(
        (pipeline, mesh, camera, deferred_textures_layout, lights_buffer): &SystemParamItem<
            Self::Params
        >
    ) -> RenderSubPassDesc {
        RenderSubPassDesc(vec![
            SubPassCommand::Pipeline(Some(pipeline.iter().next().map(|(_, p)| p.0)).flatten()),
            SubPassCommand::Mesh(mesh.0.as_ref().map(|m| m.id())),
            SubPassCommand::BindGroup(0, camera.iter().next().map(|(_, c)| c.bind_group.clone())),
            SubPassCommand::BindGroup(
                1,
                deferred_textures_layout
                    .deferred_bind_group_resolved
                    .clone()
            ),
            SubPassCommand::BindGroup(2, lights_buffer.bind_group.clone()),
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
