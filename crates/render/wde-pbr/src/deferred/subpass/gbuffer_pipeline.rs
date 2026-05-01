use bevy::{
    ecs::system::{
        SystemParamItem,
        lifetimeless::{SRes, SResMut}
    },
    prelude::*
};
use wde_camera::prelude::*;
use wde_renderer::prelude::*;

use crate::{
    deferred::subpass::{
        gbuffer_bindgroup::SsboTransformBinding, gbuffer_subpass_pbr::PushConstants
    },
    prelude::{PbrMaterial, PbrTextureFormat}
};

#[derive(TypePath, Asset, Default, Clone)]
pub(crate) struct GBufferRenderPipeline(pub CachedPipelineIndex);
impl RenderAsset for GBufferRenderPipeline {
    type SourceAsset = RenderPipelineAsset<GBufferRenderPipeline>;
    type Params = (
        SRes<AssetServer>,
        SResMut<PipelineManager>,
        SBinding<CameraBinding>,
        SBinding<SsboMeshBinding>,
        SBinding<SsboTransformBinding>,
        SBinding<PbrMaterial>
    );

    fn prepare(
        asset: Self::SourceAsset,
        (
            assets_server,
            pipeline_manager,
            camera,
            ssbo_mesh,
            gbuffer_bg,
            pbr_materials
        ): &mut SystemParamItem<Self::Params>
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        Ok(GBufferRenderPipeline(
            pipeline_manager.create_render_pipeline(
                RenderPipelineDescriptor {
                    label: "gbuffer-pbr",
                    vert: Some(assets_server.load("core/render/pbr/gbuffer.vert.wgsl")),
                    frag: Some(assets_server.load("core/render/pbr/gbuffer.frag.wgsl")),
                    bind_group_layouts: vec![
                        ssbo_mesh.iter().next().map(|(_, m)| m.layout.clone()),
                        camera.iter().next().map(|(_, c)| c.layout.clone()),
                        gbuffer_bg.iter().next().map(|(_, t)| t.layout.clone()),
                        pbr_materials.iter().next().map(|(_, m)| m.layout.clone()),
                    ],
                    depth: DepthDescriptor {
                        enabled: true,
                        write: true,
                        stencil: StencilState {
                            front: StencilFaceState {
                                compare: CompareFunction::Always,
                                pass_op: StencilOperation::Replace,
                                fail_op: StencilOperation::Keep,
                                depth_fail_op: StencilOperation::Keep
                            },
                            back: StencilFaceState {
                                compare: CompareFunction::Always,
                                pass_op: StencilOperation::Replace,
                                fail_op: StencilOperation::Keep,
                                depth_fail_op: StencilOperation::Keep
                            },
                            read_mask: 0xFF,
                            write_mask: 0xFF
                        },
                        ..Default::default()
                    },
                    render_targets: Some(vec![
                        // Same order as the PbrDeferredTextures
                        PbrTextureFormat::DEPTH,  // Depth
                        PbrTextureFormat::ALBEDO, // Albedo
                        PbrTextureFormat::NORMAL, // Normal
                        PbrTextureFormat::AO,     // AO
                    ]),
                    push_constants: vec![PushConstantDescriptor {
                        stages: ShaderStages::VERTEX,
                        offset: 0,
                        size: std::mem::size_of::<PushConstants>() as u32
                    }],
                    sample_count: MSAA_SAMPLE_COUNT,
                    vertex_buffer: false,
                    ..default()
                },
                asset
            )?
        ))
    }
}
