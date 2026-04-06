use bevy::{
    ecs::system::{
        SystemParamItem,
        lifetimeless::{SRes, SResMut}
    },
    prelude::*
};
use wde_camera::prelude::*;
use wde_renderer::prelude::*;

use crate::deferred::{
    dependencies::{PbrMaterial, SsboTransformPbr},
    subpass::gbuffer_subpass_pbr::PushConstants
};

#[derive(Default, Asset, Clone, TypePath, Debug)]
pub struct PbrGBufferRenderPipelineAsset;
impl std::fmt::Display for PbrGBufferRenderPipelineAsset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PbrGBufferRenderPipelineAsset")
    }
}

#[allow(unused)]
#[derive(Component)]
pub struct PbrGBufferRenderPipeline(pub Handle<PbrGBufferRenderPipelineAsset>);
pub struct GpuPbrGBufferRenderPipeline(pub CachedPipelineIndex);
impl RenderAsset for GpuPbrGBufferRenderPipeline {
    type SourceAsset = PbrGBufferRenderPipelineAsset;
    type Params = (
        SRes<AssetServer>,
        SResMut<PipelineManager>,
        SBinding<CameraRender>,
        SBinding<SsboMesh>,
        SMaterial<PbrMaterial>
    );

    fn prepare(
        asset: Self::SourceAsset,
        (assets_server, pipeline_manager, camera, ssbo_mesh, pbr_materials): &mut SystemParamItem<
            Self::Params
        >
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        Ok(GpuPbrGBufferRenderPipeline(
            pipeline_manager.create_render_pipeline(
                RenderPipelineDescriptor {
                    label: "gbuffer-pbr",
                    vert: Some(assets_server.load("core/render/pbr/gbuffer_vert.wgsl")),
                    frag: Some(assets_server.load("core/render/pbr/gbuffer_frag.wgsl")),
                    bind_group_layouts: vec![
                        ssbo_mesh.iter().next().map(|(_, m)| m.layout.clone()),
                        camera.iter().next().map(|(_, c)| c.layout.clone()),
                        Some(SsboTransformPbr::get_layout()),
                        pbr_materials.iter().next().map(|(_, m)| m.layout.clone()),
                    ],
                    depth: DepthDescriptor {
                        enabled: true,
                        ..default()
                    },
                    render_targets: Some(vec![
                        TextureFormat::R16Float,       // Depth
                        TextureFormat::Rgba8UnormSrgb, // Albedo
                        TextureFormat::Rgba16Float,    // Normal
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
