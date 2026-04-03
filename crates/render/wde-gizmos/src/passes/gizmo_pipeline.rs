use bevy::{ecs::system::{SystemParamItem, lifetimeless::{SRes, SResMut}}, prelude::*};
use wde_camera::prelude::*;
use wde_renderer::prelude::*;

use crate::assets::gizmo_material::GizmoMaterialAsset;

use super::GizmoSsbo;


#[derive(Default, Asset, Clone, TypePath)]
pub struct GizmoRenderPipelineAsset;
#[allow(dead_code)]
#[derive(Component)]
pub struct GizmoRenderPipeline(pub Handle<GizmoRenderPipelineAsset>);
pub struct GpuGizmoRenderPipeline(pub CachedPipelineIndex);
impl RenderAsset for GpuGizmoRenderPipeline {
    type SourceAsset = GizmoRenderPipelineAsset;
    type Param = (SRes<AssetServer>, SResMut<PipelineManager>, SRes<CameraFeatureRender>, SRes<RenderAssets<GpuMaterial<GizmoMaterialAsset>>>);

    fn prepare_asset(
            asset: Self::SourceAsset,
            (assets_server, pipeline_manager, camera_feature, materials): &mut SystemParamItem<Self::Param>
        ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        let material = match materials.iter().next() {
            Some((_, material)) => material,
            None => return Err(PrepareAssetError::RetryNextUpdate(asset))
        };

        Ok(GpuGizmoRenderPipeline(pipeline_manager.create_render_pipeline(RenderPipelineDescriptor {
            label: "gizmo",
            vert: Some(assets_server.load("core/render/gizmo/vert.wgsl")),
            frag: Some(assets_server.load("core/render/gizmo/frag.wgsl")),
            bind_group_layouts: vec![
                camera_feature.layout.clone(),
                GizmoSsbo::layout(),
                material.bind_group_layout.clone()
            ],
            depth: DepthDescriptor {
                enabled: true,
                ..Default::default()
            },
            topology: RenderTopology::LineList,
            cull_mode: None,
            ..Default::default()
        })))
    }

    fn label(&self) -> &str {
        "gizmo"
    }
}
