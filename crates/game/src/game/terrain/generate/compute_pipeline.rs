use bevy::{ecs::system::lifetimeless::{SRes, SResMut}, prelude::*};
use wde_render::{assets::{PrepareAssetError, RenderAsset}, pipelines::{CachedPipelineIndex, ComputePipelineDescriptor, PipelineManager}};
use wde_wgpu::{bind_group::{BindGroupLayout, WgpuBindGroup}, buffer::BufferBindingType, render_pipeline::ShaderStages};

#[derive(Default, Asset, Clone, TypePath)]
pub struct MCComputePipelineGenerateAsset;
#[derive(Component)]
#[allow(dead_code)]
pub struct MCComputePipelineGenerate(pub Handle<MCComputePipelineGenerateAsset>);
pub struct GpuMCComputePipelineGenerate {
    pub cached_pipeline_index: CachedPipelineIndex,

    // Bind group layouts
    pub desc_gpu_layout: Option<BindGroupLayout>,
    pub points_gpu_layout: Option<BindGroupLayout>,
    pub triangles_gpu_layout: Option<BindGroupLayout>,

    // Bind groups
    pub desc_gpu_group: Option<WgpuBindGroup>,
    pub triangles_gpu_group: Option<WgpuBindGroup>
}
impl RenderAsset for GpuMCComputePipelineGenerate {
    type SourceAsset = MCComputePipelineGenerateAsset;
    type Param = (
        SRes<AssetServer>, SResMut<PipelineManager>
    );

    fn prepare_asset(
            _asset: Self::SourceAsset,
            (
                assets_server, pipeline_manager
            ): &mut bevy::ecs::system::SystemParamItem<Self::Param>
        ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        // Create the layouts
        let desc_gpu_layout = BindGroupLayout::new("marching-cubes-generate-desc", |builder| {
            builder.add_buffer(0,
                ShaderStages::COMPUTE,
                BufferBindingType::Storage { read_only: false });
        });
        let points_gpu_layout = BindGroupLayout::new("marching-cubes-generate-points", |builder| {
            builder.add_buffer(0,
                ShaderStages::COMPUTE,
                BufferBindingType::Storage { read_only: true });
        });
        let vertices_gpu_layout = BindGroupLayout::new("marching-cubes-generate-triangles", |builder| {
            builder.add_buffer(0,
                ShaderStages::COMPUTE,
                BufferBindingType::Storage { read_only: false });
        });

        // Create the pipeline
        let pipeline_desc = ComputePipelineDescriptor {
            label: "marching-cubes-generate",
            comp: Some(assets_server.load("marching-cubes/marching_cube.comp.wgsl")),
            bind_group_layouts: vec![desc_gpu_layout.clone(), points_gpu_layout.clone(), vertices_gpu_layout.clone()],
            ..Default::default()
        };
        let cached_index = pipeline_manager.create_compute_pipeline(pipeline_desc);

        Ok(GpuMCComputePipelineGenerate {
            cached_pipeline_index: cached_index,
            desc_gpu_layout: Some(desc_gpu_layout),
            points_gpu_layout: Some(points_gpu_layout),
            triangles_gpu_layout: Some(vertices_gpu_layout),
            desc_gpu_group: None,
            triangles_gpu_group: None
        })
    }

    fn label(&self) -> &str {
        "marching-cubes-generate"
    }
}
