use bevy::{ecs::system::lifetimeless::{SRes, SResMut}, prelude::*};
use wde_render::{assets::{PrepareAssetError, RenderAsset}, pipelines::{CachedPipelineIndex, ComputePipelineDescriptor, PipelineManager}};
use wde_wgpu::{bind_group::{BindGroupLayout, WgpuBindGroup}, buffer::BufferBindingType, render_pipeline::ShaderStages};

#[derive(Default, Asset, Clone, TypePath)]
pub struct MCComputePipelineSpawnAsset;
#[derive(Component)]
#[allow(dead_code)]
pub struct MCComputePipelineSpawn(pub Handle<MCComputePipelineSpawnAsset>);
pub struct GpuMCComputePipelineSpawn {
    pub cached_pipeline_index: CachedPipelineIndex,

    // Bind group layouts
    pub desc_gpu_layout: Option<BindGroupLayout>,
    pub points_gpu_layout: Option<BindGroupLayout>,
    pub noise_gpu_layout: Option<BindGroupLayout>,

    // Bind groups
    pub desc_gpu_group: Option<WgpuBindGroup>,
    pub noise_gpu_group: Option<WgpuBindGroup>
}
impl RenderAsset for GpuMCComputePipelineSpawn {
    type SourceAsset = MCComputePipelineSpawnAsset;
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
        let desc_gpu_layout = BindGroupLayout::new("marching-cubes-spawn-desc", |builder| {
            builder.add_buffer(0,
                ShaderStages::COMPUTE,
                BufferBindingType::Storage { read_only: true });
        });
        let points_gpu_layout = BindGroupLayout::new("marching-cubes-spawn-points", |builder| {
            builder.add_buffer(0,
                ShaderStages::COMPUTE,
                BufferBindingType::Storage { read_only: false });
        });
        let noise_gpu_layout = BindGroupLayout::new("marching-cubes-spawn-noise", |builder| {
            builder.add_buffer(0,
                ShaderStages::COMPUTE,
                BufferBindingType::Uniform);
        });

        // Create the pipeline
        let pipeline_desc = ComputePipelineDescriptor {
            label: "marching-cubes",
            comp: Some(assets_server.load("marching-cubes/spawn_terrain.comp.wgsl")),
            bind_group_layouts: vec![desc_gpu_layout.clone(), points_gpu_layout.clone(), noise_gpu_layout.clone()],
            ..Default::default()
        };
        let cached_index = pipeline_manager.create_compute_pipeline(pipeline_desc);

        Ok(GpuMCComputePipelineSpawn {
            cached_pipeline_index: cached_index,
            desc_gpu_layout: Some(desc_gpu_layout),
            points_gpu_layout: Some(points_gpu_layout),
            noise_gpu_layout: Some(noise_gpu_layout),
            desc_gpu_group: None,
            noise_gpu_group: None
        })
    }

    fn label(&self) -> &str {
        "marching-cubes-spawn"
    }
}
