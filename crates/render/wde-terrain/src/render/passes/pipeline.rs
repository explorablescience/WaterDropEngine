use bevy::{ecs::system::lifetimeless::{SRes, SResMut}, prelude::*};
use wde_camera::features::CameraFeatureRender;
use wde_renderer::prelude::*;


#[derive(Default, Asset, Clone, TypePath)]
pub(crate) struct TerrainRenderPipelineAsset;

#[allow(unused)]
#[derive(Component)]
pub(crate) struct TerrainRenderPipeline(pub Handle<TerrainRenderPipelineAsset>);
pub(crate) struct GpuTerrainRenderPipeline {
    pub cached_pipeline_index: CachedPipelineIndex
}
impl RenderAsset for GpuTerrainRenderPipeline {
    type SourceAsset = TerrainRenderPipelineAsset;
    type Param = (
        SRes<AssetServer>, SResMut<PipelineManager>, SRes<CameraFeatureRender>
    );

    fn prepare_asset(
            _asset: Self::SourceAsset,
            (
                assets_server, pipeline_manager, camera_feature
            ): &mut bevy::ecs::system::SystemParamItem<Self::Param>
        ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        // Create the pipeline
        let pipeline_desc = RenderPipelineDescriptor {
            label: "terrain",
            vert: Some(assets_server.load("core/render/terrain/render_terrain_vert.wgsl")),
            frag: Some(assets_server.load("core/render/terrain/render_terrain_frag.wgsl")),
            bind_group_layouts: vec![camera_feature.layout.clone()],
            ..Default::default()
        };
        let cached_index = pipeline_manager.create_render_pipeline(pipeline_desc);

        Ok(GpuTerrainRenderPipeline {
            cached_pipeline_index: cached_index
        })
    }

    fn label(&self) -> &str {
        "terrain"
    }
}
