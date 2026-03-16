use bevy::{ecs::system::lifetimeless::{SRes, SResMut}, prelude::*};
use wde::prelude::*;

use super::{CustomMaterialAsset, CustomSsbo};


#[derive(Default, Asset, Clone, TypePath)]
/// Render the entities with a custom material and a mesh.
pub struct CustomRenderPipelineAsset;

#[allow(dead_code)]
#[derive(Component)]
pub struct CustomRenderPipeline(pub Handle<CustomRenderPipelineAsset>);

/// Represents the gpu custom render pipeline.
pub struct GpuCustomRenderPipeline {
    pub cached_pipeline_index: CachedPipelineIndex
}
impl RenderAsset for GpuCustomRenderPipeline {
    type SourceAsset = CustomRenderPipelineAsset;
    type Param = (
        SRes<AssetServer>, SResMut<PipelineManager>,
        SRes<CameraFeatureRender>, SRes<RenderAssets<GpuMaterial<CustomMaterialAsset>>>, SRes<CustomSsbo>
    );

    fn prepare_asset(
            asset: Self::SourceAsset,
            (
                assets_server, pipeline_manager,
                camera_feature, materials, ssbo
            ): &mut bevy::ecs::system::SystemParamItem<Self::Param>
        ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        // Get the ssbo layout
        let ssbo_layout = match &ssbo.bind_group_layout {
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
            label: "custom",
            vert: Some(assets_server.load("examples/custom_forward_render/vert.wgsl")),
            frag: Some(assets_server.load("examples/custom_forward_render/frag.wgsl")),
            bind_group_layouts: vec![camera_feature.layout.clone(), ssbo_layout.clone(), material.bind_group_layout.clone()],
            depth: DepthDescriptor {
                enabled: true,
                ..Default::default()
            },
            render_targets: None,
            ..Default::default()
        };
        let cached_index = pipeline_manager.create_render_pipeline(pipeline_desc);

        Ok(GpuCustomRenderPipeline {
            cached_pipeline_index: cached_index
        })
    }

    fn label(&self) -> &str {
        "custom"
    }
}
