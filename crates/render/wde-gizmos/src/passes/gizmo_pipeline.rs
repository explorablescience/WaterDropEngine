use bevy::{ecs::system::lifetimeless::{SRes, SResMut}, prelude::*};
use wde_camera::prelude::*;
use wde_renderer::prelude::*;

use crate::assets::gizmo_material::GizmoMaterialAsset;

use super::GizmoSsbo;


#[derive(Default, Asset, Clone, TypePath)]
pub(crate) struct GizmoRenderPipelineAsset;

#[allow(dead_code)]
#[derive(Component)]
pub(crate) struct GizmoRenderPipeline(pub Handle<GizmoRenderPipelineAsset>);
pub(crate) struct GpuGizmoRenderPipeline {
    pub cached_pipeline_index: CachedPipelineIndex
}
impl RenderAsset for GpuGizmoRenderPipeline {
    type SourceAsset = GizmoRenderPipelineAsset;
    type Param = (
        SRes<AssetServer>, SResMut<PipelineManager>, SRes<CameraFeatureRender>,
        SRes<RenderAssets<GpuMaterial<GizmoMaterialAsset>>>, SRes<GizmoSsbo>
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
            label: "gizmo",
            vert: Some(assets_server.load("core/render/gizmo/vert.wgsl")),
            frag: Some(assets_server.load("core/render/gizmo/frag.wgsl")),
            bind_group_layouts: vec![camera_feature.layout.clone(), ssbo_layout.clone(), material.bind_group_layout.clone()],
            depth: DepthDescriptor {
                enabled: true,
                ..Default::default()
            },
            topology: RenderTopology::LineList,
            cull_mode: None,
            ..Default::default()
        };
        let cached_index = pipeline_manager.create_render_pipeline(pipeline_desc);

        Ok(GpuGizmoRenderPipeline {
            cached_pipeline_index: cached_index
        })
    }

    fn label(&self) -> &str {
        "gizmo"
    }
}
