use bevy::{ecs::system::lifetimeless::{SRes, SResMut}, prelude::*};
use wde_camera::prelude::*;
use wde_renderer::{MSAA_SAMPLE_COUNT, pipelines::PushConstantDescriptor, prelude::*, ssbos::ssbo_mesh::SsboMesh};

use crate::{assets::PbrMaterialAsset, logic::{ssbo::PbrSsbo, textures::PbrDeferredTextures}, passes::PushConstants};


#[derive(Default, Asset, Clone, TypePath)]
pub(crate) struct PbrGBufferRenderPipelineAsset;

#[allow(unused)]
#[derive(Component)]
pub(crate) struct PbrGBufferRenderPipeline(pub Handle<PbrGBufferRenderPipelineAsset>);
pub(crate) struct GpuPbrGBufferRenderPipeline {
    pub cached_pipeline_index: CachedPipelineIndex
}
impl RenderAsset for GpuPbrGBufferRenderPipeline {
    type SourceAsset = PbrGBufferRenderPipelineAsset;
    type Param = (
        SRes<AssetServer>, SResMut<PipelineManager>,
        SRes<CameraFeatureRender>, SRes<RenderAssets<GpuMaterial<PbrMaterialAsset>>>, SRes<PbrSsbo>,
        SRes<PbrDeferredTextures>, SRes<RenderAssets<GpuTexture>>, SRes<SsboMesh>
    );

    fn prepare_asset(
            asset: Self::SourceAsset,
            (
                assets_server, pipeline_manager,
                camera_feature, materials, ssbo,
                defered_textures, textures, mesh_ssbo
            ): &mut bevy::ecs::system::SystemParamItem<Self::Param>
        ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        // Get the defered textures
        let (depth, albedo, normal) =
            match (textures.get(&defered_textures.depth),
                   textures.get(&defered_textures.albedo),
                   textures.get(&defered_textures.normal)
            ) {
                (Some(depth), Some(albedo), Some(normal))
                    => (depth, albedo, normal),
                _ => return Err(PrepareAssetError::RetryNextUpdate(asset))
            };

        // Get the ssbo layout
        let ssbo_layout = match &ssbo.bind_group_layout {
            Some(layout) => layout,
            None => return Err(PrepareAssetError::RetryNextUpdate(asset))
        };

        // Get the mesh ssbo layout
        let mesh_ssbo_layout = match &mesh_ssbo.bind_group_layout {
            Some(layout) => layout,
            None => return Err(PrepareAssetError::RetryNextUpdate(asset))
        };

        // Get the material layout
        let material = match materials.iter().next() {
            Some((_, material)) => material,
            None => return Err(PrepareAssetError::RetryNextUpdate(asset))
        };

        // Create the pipeline
        let pipeline_desc = RenderPipelineDescriptor {
            label: "gbuffer-pbr",
            vert: Some(assets_server.load("core/render/pbr/gbuffer_vert.wgsl")),
            frag: Some(assets_server.load("core/render/pbr/gbuffer_frag.wgsl")),
            bind_group_layouts: vec![mesh_ssbo_layout.clone(), camera_feature.layout.clone(), ssbo_layout.clone(), material.bind_group_layout.clone()],
            depth: DepthStencilDescriptor {
                enabled: true,
                ..Default::default()
            },
            render_targets: Some(vec![
                depth.texture.format, albedo.texture.format, normal.texture.format
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
            ..Default::default()
        };
        let cached_index = pipeline_manager.create_render_pipeline(pipeline_desc);

        Ok(GpuPbrGBufferRenderPipeline {
            cached_pipeline_index: cached_index
        })
    }

    fn label(&self) -> &str {
        "gbuffer-pbr"
    }
}
