use wde_logger::prelude::*;

use bevy::{
    ecs::system::{
        SystemParamItem,
        lifetimeless::{SRes, SResMut}
    },
    prelude::*
};
use wde_renderer::{prelude::*, wgpu_utils::*};

use super::data::PostProcessBuffers;
use crate::prelude::*;

/// Workgroup size of the histogram shader, in pixels per side.
const HISTOGRAM_TILE: u32 = 16;

/// Eye adaptation: bins the luminance of the [`RenderTexture`] into a histogram, then moves the adapted
/// exposure towards the one bringing its average (within the metering percentiles) to middle grey.
///
/// The adapted exposure stays on the GPU and is applied by the composite pass. It has a render index of 92,
/// and does nothing when [`ExposureSettings::eye_adaptation`] is disabled.
pub struct RenderPassExposure;
impl RenderPass for RenderPassExposure {
    type Params = ();

    fn describe(_: &SystemParamItem<Self::Params>) -> RenderPassDesc {
        RenderPassDesc::default()
    }

    fn id() -> RenderPassId {
        92
    }
    fn label() -> &'static str {
        "exposure"
    }

    fn custom_render(world: &mut World, command_buffer: &mut CommandBuffer) -> Option<bool> {
        if world.resource::<PostProcessUniform>().exposure[0] <= 0.0 {
            return Some(true);
        }
        Some(
            world.resource_scope(|world, mut bind_group: Mut<ExposureBindGroup>| {
                bind_group.update(world) && render_exposure(world, &bind_group, command_buffer)
            })
        )
    }
}

fn exposure_layout() -> BindGroupLayout {
    BindGroupLayout::new("exposure", |builder| {
        let storage = BufferBindingType::Storage { read_only: false };
        builder
            .add_texture_view_filterable(0, ShaderStages::COMPUTE, false, true)
            .add_buffer(1, ShaderStages::COMPUTE, BufferBindingType::Uniform)
            .add_buffer(2, ShaderStages::COMPUTE, storage)
            .add_buffer(3, ShaderStages::COMPUTE, storage);
    })
}

/// Bind group of the exposure shaders: the render texture, the parameters, the histogram and the exposure.
#[derive(Resource, Default)]
pub(crate) struct ExposureBindGroup {
    texture: Option<AssetId<Texture>>,
    size: (u32, u32),
    bind_group: Option<BindGroup>
}
impl ExposureBindGroup {
    /// Recreates the bind group if the render texture changed. Returns false if it is not ready.
    fn update(&mut self, world: &World) -> bool {
        let Some(texture) = world
            .resource::<RenderAssets<GpuRenderData<RenderTexture>>>()
            .iter()
            .next()
            .and_then(|(_, t)| t.get_texture(RenderTexture::BINDING))
            .map(|t| t.id())
        else {
            return false;
        };
        if self.texture == Some(texture) && self.bind_group.is_some() {
            return true;
        }

        let Some((_, data)) = world
            .resource::<RenderAssets<GpuRenderData<PostProcessBuffers>>>()
            .iter()
            .next()
        else {
            return false;
        };
        let gpu_buffers = world.resource::<RenderAssets<GpuBuffer>>();
        let buffer = |idx| data.get_buffer(idx).and_then(|b| gpu_buffers.get(&b));
        let (Some(params), Some(histogram), Some(exposure)) = (
            buffer(PostProcessBuffers::PARAMS_IDX),
            buffer(PostProcessBuffers::HISTOGRAM_IDX),
            buffer(PostProcessBuffers::EXPOSURE_IDX)
        ) else {
            return false;
        };
        let Some(gpu_texture) = world.resource::<RenderAssets<GpuTexture>>().get(texture) else {
            return false;
        };

        let instance = world.resource::<RenderInstance>().0.read().unwrap();
        let Ok(layout) = exposure_layout().build(&instance) else {
            return false;
        };
        let Ok(bind_group) = BindGroupBuilder::build(
            "exposure",
            &instance,
            &layout,
            &vec![
                BindGroupBuilder::texture_view(0, &gpu_texture.texture),
                BindGroupBuilder::buffer(1, &params.buffer),
                BindGroupBuilder::buffer(2, &histogram.buffer),
                BindGroupBuilder::buffer(3, &exposure.buffer),
            ]
        ) else {
            return false;
        };
        self.texture = Some(texture);
        self.size = gpu_texture.texture.size;
        self.bind_group = Some(bind_group);
        true
    }
}

fn render_exposure(
    world: &World,
    bind_group: &ExposureBindGroup,
    command_buffer: &mut CommandBuffer
) -> bool {
    let pipeline_manager = world.resource::<PipelineManager>();
    let pipeline =
        |id: Option<CachedPipelineIndex>| match id.map(|id| pipeline_manager.get_pipeline(id)) {
            Some(CachedPipelineStatus::OkCompute(p)) => Some(p),
            _ => None
        };
    let (Some(histogram), Some(average), Some(bind_group_gpu)) = (
        pipeline(
            world
                .resource::<RenderAssets<ExposureHistogramPipeline>>()
                .iter()
                .next()
                .map(|(_, p)| p.0)
        ),
        pipeline(
            world
                .resource::<RenderAssets<ExposureAveragePipeline>>()
                .iter()
                .next()
                .map(|(_, p)| p.0)
        ),
        bind_group.bind_group.as_ref()
    ) else {
        return false;
    };

    // Separate dispatches: the average reads the histogram written by every workgroup
    let mut pass = command_buffer.create_compute_pass("exposure");
    let (width, height) = bind_group.size;
    for (pipeline, workgroups) in [
        (
            histogram,
            (
                width.div_ceil(HISTOGRAM_TILE),
                height.div_ceil(HISTOGRAM_TILE)
            )
        ),
        (average, (1, 1))
    ] {
        if pass.set_pipeline(pipeline).is_err() {
            return false;
        }
        pass.set_bind_group(0, bind_group_gpu);
        if let Err(e) = pass.dispatch(workgroups.0, workgroups.1, 1) {
            debug!("Failed to dispatch the exposure compute: {:?}.", e);
            return false;
        }
    }
    true
}

macro_rules! exposure_pipeline {
    ($name:ident, $label:literal, $shader:literal) => {
        #[derive(TypePath, Default, Clone, Debug)]
        pub(crate) struct $name(pub CachedPipelineIndex);
        impl RenderAsset for $name {
            type SourceAsset = RenderPipelineAsset<$name>;
            type Params = (SRes<AssetServer>, SResMut<PipelineManager>);

            fn prepare(
                _id: AssetId<Self::SourceAsset>,
                asset: Self::SourceAsset,
                (assets_server, pipeline_manager): &mut SystemParamItem<Self::Params>
            ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
                Ok($name(pipeline_manager.create_compute_pipeline(
                    ComputePipelineDescriptor {
                        label: $label,
                        comp: Some(assets_server.load($shader)),
                        bind_group_layouts: vec![Some(exposure_layout())],
                        push_constants: vec![]
                    },
                    asset
                )?))
            }
        }
    };
}
exposure_pipeline!(
    ExposureHistogramPipeline,
    "exposure-histogram",
    "core/render/post_process/exposure_histogram.comp.wgsl"
);
exposure_pipeline!(
    ExposureAveragePipeline,
    "exposure-average",
    "core/render/post_process/exposure_average.comp.wgsl"
);
