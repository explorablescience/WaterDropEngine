use bevy::{ecs::system::lifetimeless::{SRes, SResMut}, prelude::*};
use wde_camera::prelude::*;
use wde_renderer::prelude::*;


#[derive(Default, Asset, Clone, TypePath)]
pub(crate) struct SelectedObjectRenderPipelineAsset;

#[allow(unused)]
#[derive(Component)]
pub(crate) struct SelectedObjectRenderPipeline(pub Handle<SelectedObjectRenderPipelineAsset>);
pub(crate) struct GpuSelectedObjectRenderPipeline {
    pub cached_pipeline_index: CachedPipelineIndex
}
impl RenderAsset for GpuSelectedObjectRenderPipeline {
    type SourceAsset = SelectedObjectRenderPipelineAsset;
    type Param = (
        SRes<AssetServer>, SResMut<PipelineManager>, SRes<CameraFeatureRender>
    );

    fn prepare_asset(
            _asset: Self::SourceAsset,
            (
                assets_server, pipeline_manager, camera_feature
            ): &mut bevy::ecs::system::SystemParamItem<Self::Param>
        ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        let pipeline_desc = RenderPipelineDescriptor {
            label: "selected-object",
            vert: Some(assets_server.load("core/render/terrain/render_selected_object_vert.wgsl")),
            frag: Some(assets_server.load("core/render/terrain/render_selected_object_frag.wgsl")),
            bind_group_layouts: vec![
                camera_feature.layout.clone()
            ],
            depth: DepthDescriptor {
                enabled: true,
                write: true,
                compare: CompareFunction::Always,
                stencil: StencilState {
                    front: StencilFaceState::default(),
                    back: StencilFaceState::default(),
                    read_mask: 0xFF,
                    write_mask: 0x01
                }
            },
            ..Default::default()
        };
        Ok(GpuSelectedObjectRenderPipeline {
            cached_pipeline_index: pipeline_manager.create_render_pipeline(pipeline_desc)
        })
    }

    fn label(&self) -> &str {
        "selected-object"
    }
}
