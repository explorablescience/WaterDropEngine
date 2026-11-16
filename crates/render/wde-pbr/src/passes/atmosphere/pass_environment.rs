use wde_logger::prelude::*;

use bevy::{
    ecs::system::{
        SystemParamItem,
        lifetimeless::{SRes, SResMut}
    },
    prelude::*
};
use wde_renderer::{prelude::*, wgpu_utils::*};

use crate::prelude::*;

/// The environment is baked again when the sun moved by more than this angle, in radians.
const REBAKE_SUN_ANGLE: f32 = 0.25 * std::f32::consts::PI / 180.0;
/// Workgroup size of the prefilter shader, in texels per side.
const PREFILTER_TILE: u32 = 8;

/// Bakes the [`EnvironmentData`] from the sky: projects it on spherical harmonics for the diffuse lighting, and
/// prefilters it into the mips of the environment map for the specular lighting.
///
/// It only runs when the sun moved by more than a fraction of a degree, or when the other [`AtmosphereSettings`]
/// changed. It has a render index of 4, before the shadows and the lighting.
pub struct RenderPassEnvironment;
impl RenderPass for RenderPassEnvironment {
    type Params = ();

    fn describe(_: &SystemParamItem<Self::Params>) -> RenderPassDesc {
        RenderPassDesc::default()
    }

    fn id() -> RenderPassId {
        4
    }
    fn label() -> &'static str {
        "environment"
    }

    fn custom_render(world: &mut World, command_buffer: &mut CommandBuffer) -> Option<bool> {
        let params = *world.resource::<AtmosphereParamsUniform>();
        if !world.resource::<EnvironmentBake>().needs_bake(&params) {
            return Some(true);
        }
        Some(
            world.resource_scope(|world, mut bake: Mut<EnvironmentBake>| {
                let baked = bake.update(world) && bake_environment(world, &bake, command_buffer);
                if baked {
                    bake.baked = Some(params);
                }
                baked
            })
        )
    }
}

fn environment_layout() -> BindGroupLayout {
    BindGroupLayout::new("environment-bake", |builder| {
        builder
            .add_buffer(0, ShaderStages::COMPUTE, BufferBindingType::Uniform)
            .add_buffer(
                1,
                ShaderStages::COMPUTE,
                BufferBindingType::Storage { read_only: false }
            )
            .add_write_storage_texture_array_view(2, ENVIRONMENT_MAP_FORMAT);
    })
}

/// Bind groups of the environment shaders, one per mip of the environment map: the atmosphere parameters, the
/// spherical harmonics, and the mip written.
#[derive(Resource, Default)]
pub(crate) struct EnvironmentBake {
    map: Option<AssetId<Texture>>,
    size: u32,
    bind_groups: Vec<BindGroup>,
    /// Atmosphere parameters of the last bake.
    baked: Option<AtmosphereParamsUniform>
}
impl EnvironmentBake {
    fn needs_bake(&self, params: &AtmosphereParamsUniform) -> bool {
        let Some(baked) = self.baked else {
            return true;
        };
        let sun = |p: &AtmosphereParamsUniform| AtmosphereSettings::sun_direction(p.time_params[0]);
        if sun(&baked).angle_between(sun(params)) > REBAKE_SUN_ANGLE {
            return true;
        }
        let without_time = |p: &AtmosphereParamsUniform| {
            let mut p = *p;
            p.time_params[0] = 0.0;
            p
        };
        bytemuck::bytes_of(&without_time(&baked)) != bytemuck::bytes_of(&without_time(params))
    }

    /// Recreates the bind groups if the environment map changed. Returns false if it is not ready.
    fn update(&mut self, world: &World) -> bool {
        let Some((_, environment)) = world
            .resource::<RenderAssets<GpuRenderData<EnvironmentData>>>()
            .iter()
            .next()
        else {
            return false;
        };
        let Some(map) = environment
            .get_texture(EnvironmentData::MAP_IDX)
            .map(|t| t.id())
        else {
            return false;
        };
        if self.map == Some(map) && !self.bind_groups.is_empty() {
            return true;
        }

        let gpu_buffers = world.resource::<RenderAssets<GpuBuffer>>();
        let Some(sh) = environment
            .get_buffer(EnvironmentData::SH_BUFFER_IDX)
            .and_then(|b| gpu_buffers.get(&b))
        else {
            return false;
        };
        let Some(atmosphere) = world
            .resource::<RenderAssets<GpuRenderData<LightsData>>>()
            .iter()
            .next()
            .and_then(|(_, l)| l.get_buffer(LightsData::ATMOSPHERE_PARAMS_BUFFER_IDX))
            .and_then(|b| gpu_buffers.get(&b))
        else {
            return false;
        };
        let Some(gpu_map) = world.resource::<RenderAssets<GpuTexture>>().get(map) else {
            return false;
        };

        let instance = world.resource::<RenderInstance>().0.read().unwrap();
        let Ok(layout) = environment_layout().build(&instance) else {
            return false;
        };
        let mut bind_groups = Vec::with_capacity(gpu_map.texture.mip_level_count as usize);
        for mip in 0..gpu_map.texture.mip_level_count {
            let view = gpu_map.texture.mip_view(mip);
            let Ok(bind_group) = BindGroupBuilder::build(
                "environment-bake",
                &instance,
                &layout,
                &vec![
                    BindGroupBuilder::buffer(0, &atmosphere.buffer),
                    BindGroupBuilder::buffer(1, &sh.buffer),
                    BindGroupBuilder::view(2, &view),
                ]
            ) else {
                return false;
            };
            bind_groups.push(bind_group);
        }
        self.map = Some(map);
        self.size = gpu_map.texture.size.0;
        self.bind_groups = bind_groups;
        true
    }
}

/// Push constants of the prefilter shader.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct PrefilterParams {
    roughness: f32
}

fn bake_environment(
    world: &World,
    bake: &EnvironmentBake,
    command_buffer: &mut CommandBuffer
) -> bool {
    let pipeline_manager = world.resource::<PipelineManager>();
    let pipeline =
        |id: Option<CachedPipelineIndex>| match id.map(|id| pipeline_manager.get_pipeline(id)) {
            Some(CachedPipelineStatus::OkCompute(p)) => Some(p),
            _ => None
        };
    let (Some(sh), Some(prefilter)) = (
        pipeline(
            world
                .resource::<RenderAssets<EnvironmentShPipeline>>()
                .iter()
                .next()
                .map(|(_, p)| p.0)
        ),
        pipeline(
            world
                .resource::<RenderAssets<EnvironmentPrefilterPipeline>>()
                .iter()
                .next()
                .map(|(_, p)| p.0)
        )
    ) else {
        return false;
    };

    let mut pass = command_buffer.create_compute_pass("environment");
    if pass.set_pipeline(sh).is_err() {
        return false;
    }
    pass.set_bind_group(0, &bake.bind_groups[0]);
    if let Err(e) = pass.dispatch(1, 1, 1) {
        trace!("Failed to dispatch the environment harmonics: {:?}.", e);
        return false;
    }

    if pass.set_pipeline(prefilter).is_err() {
        return false;
    }
    let last_mip = (bake.bind_groups.len() - 1).max(1) as f32;
    for (mip, bind_group) in bake.bind_groups.iter().enumerate() {
        let size = (bake.size >> mip).max(1);
        let params = PrefilterParams {
            roughness: mip as f32 / last_mip
        };
        pass.set_bind_group(0, bind_group)
            .set_push_constants(bytemuck::bytes_of(&params));
        let workgroups = size.div_ceil(PREFILTER_TILE);
        if let Err(e) = pass.dispatch(workgroups, workgroups, 6) {
            trace!("Failed to dispatch the environment prefilter: {:?}.", e);
            return false;
        }
    }
    drop(pass);
    if let Some(map) = bake.map {
        RenderGraphDebug::capture(world, command_buffer, "prefiltered map", &[map]);
    }
    true
}

#[derive(TypePath, Default, Clone, Debug)]
pub(crate) struct EnvironmentShPipeline(pub CachedPipelineIndex);
impl RenderAsset for EnvironmentShPipeline {
    type SourceAsset = RenderPipelineAsset<EnvironmentShPipeline>;
    type Params = (SRes<AssetServer>, SResMut<PipelineManager>);

    fn prepare(
        _id: AssetId<Self::SourceAsset>,
        asset: Self::SourceAsset,
        (assets_server, pipeline_manager): &mut SystemParamItem<Self::Params>
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        Ok(EnvironmentShPipeline(
            pipeline_manager.create_compute_pipeline(
                ComputePipelineDescriptor {
                    label: "environment-sh",
                    comp: Some(
                        assets_server.load("core/render/atmosphere/environment_sh.comp.wgsl")
                    ),
                    bind_group_layouts: vec![Some(environment_layout())],
                    push_constants: vec![]
                },
                asset
            )?
        ))
    }
}

#[derive(TypePath, Default, Clone, Debug)]
pub(crate) struct EnvironmentPrefilterPipeline(pub CachedPipelineIndex);
impl RenderAsset for EnvironmentPrefilterPipeline {
    type SourceAsset = RenderPipelineAsset<EnvironmentPrefilterPipeline>;
    type Params = (SRes<AssetServer>, SResMut<PipelineManager>);

    fn prepare(
        _id: AssetId<Self::SourceAsset>,
        asset: Self::SourceAsset,
        (assets_server, pipeline_manager): &mut SystemParamItem<Self::Params>
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        Ok(EnvironmentPrefilterPipeline(
            pipeline_manager.create_compute_pipeline(
                ComputePipelineDescriptor {
                    label: "environment-prefilter",
                    comp: Some(
                        assets_server
                            .load("core/render/atmosphere/environment_prefilter.comp.wgsl")
                    ),
                    bind_group_layouts: vec![Some(environment_layout())],
                    push_constants: vec![PushConstantDescriptor {
                        offset: 0,
                        size: std::mem::size_of::<PrefilterParams>() as u32
                    }]
                },
                asset
            )?
        ))
    }
}
