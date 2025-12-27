use bevy::{ecs::system::{SystemParamItem, lifetimeless::{SRes, SResMut}}, prelude::*};
use wde_render::{assets::{PrepareAssetError, RenderAsset}, pipelines::{CachedPipelineIndex, PipelineManager, RenderPipelineDescriptor}};
use wde_wgpu::{bind_group::BindGroupLayout, render_pipeline::WShaderStages};


#[derive(Default, Asset, Clone, TypePath)]
pub struct DisplayTexturePipelineAsset;
#[allow(dead_code)]
#[derive(Component)]
pub struct DisplayTexturePipeline(pub Handle<DisplayTexturePipelineAsset>);
pub struct GpuDisplayTexturePipeline {
    pub cached_pipeline_index: CachedPipelineIndex
}
impl RenderAsset for GpuDisplayTexturePipeline {
    type SourceAsset = DisplayTexturePipelineAsset;
    type Param = (
        SRes<AssetServer>, SResMut<PipelineManager>
    );

    fn prepare_asset(
            _asset: Self::SourceAsset,
            (assets_server, pipeline_manager): &mut SystemParamItem<Self::Param>
        ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        // Create the layout
        let layout = BindGroupLayout::new("display-texture", |builder| {
            // Set the texture view and sampler
            builder.add_texture_view(0, WShaderStages::FRAGMENT);
            builder.add_texture_sampler(1, WShaderStages::FRAGMENT);
        });

        // Create the pipeline
        let pipeline_desc = RenderPipelineDescriptor {
            label: "display-texture",
            vert: Some(assets_server.load("examples/display_texture/vert.wgsl")),
            frag: Some(assets_server.load("examples/display_texture/frag.wgsl")),
            bind_group_layouts: vec![layout.clone()],
            ..Default::default()
        };

        let cached_index = pipeline_manager.create_render_pipeline(pipeline_desc);
        Ok(GpuDisplayTexturePipeline { cached_pipeline_index: cached_index })
    }

    fn label(&self) -> &str {
        "display-texture"
    }
}
