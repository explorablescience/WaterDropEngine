use bevy::{ecs::system::{SystemParamItem, lifetimeless::{SRes, SResMut}}, prelude::*};
use wde_camera::prelude::*;
use wde_renderer::{MSAA_SAMPLE_COUNT, pipelines::PushConstantDescriptor, prelude::*, ssbos::ssbo_mesh::SsboMesh};

use crate::{assets::PbrMaterialAsset, logic::ssbo::SsboTransformPbr, passes::subpass::gbuffer_subpass_pbr::PushConstants};


#[derive(Default, Asset, Clone, TypePath)]
pub struct PbrGBufferRenderPipelineAsset;

#[allow(unused)]
#[derive(Component)]
pub struct PbrGBufferRenderPipeline(pub Handle<PbrGBufferRenderPipelineAsset>);
pub struct GpuPbrGBufferRenderPipeline(pub CachedPipelineIndex);
impl RenderAsset for GpuPbrGBufferRenderPipeline {
    type SourceAsset = PbrGBufferRenderPipelineAsset;
    type Param = (SRes<AssetServer>, SResMut<PipelineManager>, SRes<CameraFeatureRender>, SRes<RenderAssets<GpuMaterial<PbrMaterialAsset>>>);

    fn prepare_asset(
            asset: Self::SourceAsset,
            (assets_server, pipeline_manager, camera_feature, materials): &mut SystemParamItem<Self::Param>
        ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        let material = match materials.iter().next() {
            Some((_, material)) => material,
            None => return Err(PrepareAssetError::RetryNextUpdate(asset))
        };

        Ok(GpuPbrGBufferRenderPipeline(pipeline_manager.create_render_pipeline(RenderPipelineDescriptor {
            label: "gbuffer-pbr",
            vert: Some(assets_server.load("core/render/pbr/gbuffer_vert.wgsl")),
            frag: Some(assets_server.load("core/render/pbr/gbuffer_frag.wgsl")),
            bind_group_layouts: vec![
                SsboMesh::layout(),
                camera_feature.layout.clone(),
                SsboTransformPbr::get_layout(),
                material.bind_group_layout.clone()
            ],
            depth: DepthDescriptor {
                enabled: true,
                ..default()
            },
            render_targets: Some(vec![
                TextureFormat::R16Float,       // Depth
                TextureFormat::Rgba8UnormSrgb, // Albedo
                TextureFormat::Rgba16Float     // Normal
            ]),
            push_constants: vec![
                PushConstantDescriptor {
                    stages: ShaderStages::VERTEX,
                    offset: 0,
                    size: std::mem::size_of::<PushConstants>() as u32
                }
            ],
            sample_count: MSAA_SAMPLE_COUNT,
            vertex_buffer: false,
            ..default()
        })))
    }
}
