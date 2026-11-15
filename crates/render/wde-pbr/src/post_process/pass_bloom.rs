use wde_logger::prelude::*;

use bevy::{
    ecs::system::{
        SystemParamItem,
        lifetimeless::{SRes, SResMut}
    },
    prelude::*
};
use wde_renderer::{prelude::*, wgpu_utils::*};

use super::data::{BLOOM_MIP_COUNT, PostProcessParamsBinding};
use crate::prelude::*;

/// Bloom: downsamples the [`RenderTexture`] through the bloom mips (thresholding the first one), then
/// upsamples them back to the first mip, which is added to the image in the composite pass.
///
/// It has a render index of 91, and does nothing when the bloom is disabled.
pub struct RenderPassBloom;
impl RenderPass for RenderPassBloom {
    type Params = ();

    fn describe(_: &SystemParamItem<Self::Params>) -> RenderPassDesc {
        RenderPassDesc::default()
    }

    fn id() -> RenderPassId {
        91
    }
    fn label() -> &'static str {
        "bloom"
    }

    fn custom_render(world: &mut World, command_buffer: &mut CommandBuffer) -> Option<bool> {
        let params = *world.resource::<PostProcessUniform>();
        if params.bloom[3] <= 0.0 {
            return Some(true);
        }
        Some(
            world.resource_scope(|world, mut bind_groups: Mut<BloomBindGroups>| {
                bind_groups.update(world)
                    && render_bloom(world, &bind_groups, command_buffer, params.bloom[2])
            })
        )
    }
}

/// Layout of the texture sampled by a bloom pass.
fn bloom_source_layout() -> BindGroupLayout {
    BindGroupLayout::new("bloom-source", |builder| {
        builder
            .add_texture_view_filterable(0, ShaderStages::all(), false, true)
            .add_texture_sampler(1, ShaderStages::all());
    })
}

/// Bind groups sampling each bloom source: the render texture, then each bloom mip.
#[derive(Resource, Default)]
pub(crate) struct BloomBindGroups {
    textures: Vec<AssetId<Texture>>,
    sources: Vec<BindGroup>
}
impl BloomBindGroups {
    /// Recreates the bind groups if the textures changed. Returns false if they are not ready.
    fn update(&mut self, world: &World) -> bool {
        let Some((_, data)) = world
            .resource::<RenderAssets<GpuRenderData<PostProcessTextures>>>()
            .iter()
            .next()
        else {
            return false;
        };
        let render_texture = world
            .resource::<RenderAssets<GpuRenderData<RenderTexture>>>()
            .iter()
            .next()
            .and_then(|(_, t)| t.get_texture(RenderTexture::BINDING));
        let Some(ids) = std::iter::once(render_texture)
            .chain(
                (0..BLOOM_MIP_COUNT).map(|i| data.get_texture(PostProcessTextures::BLOOM_IDX + i))
            )
            .map(|t| t.map(|t| t.id()))
            .collect::<Option<Vec<_>>>()
        else {
            return false;
        };
        if ids == self.textures {
            return true;
        }

        let gpu_textures = world.resource::<RenderAssets<GpuTexture>>();
        let Some(textures) = ids
            .iter()
            .map(|id| gpu_textures.get(*id))
            .collect::<Option<Vec<_>>>()
        else {
            return false;
        };
        let instance = world.resource::<RenderInstance>().0.read().unwrap();
        let Ok(layout) = bloom_source_layout().build(&instance) else {
            return false;
        };
        let Ok(sources) = textures
            .iter()
            .map(|t| {
                BindGroupBuilder::build(
                    "bloom-source",
                    &instance,
                    &layout,
                    &vec![
                        BindGroupBuilder::texture_view(0, &t.texture),
                        BindGroupBuilder::texture_sampler(1, &t.texture),
                    ]
                )
            })
            .collect::<Result<Vec<_>, _>>()
        else {
            return false;
        };
        self.textures = ids;
        self.sources = sources;
        true
    }
}

fn render_bloom(
    world: &World,
    bind_groups: &BloomBindGroups,
    command_buffer: &mut CommandBuffer,
    scatter: f32
) -> bool {
    let pipeline_manager = world.resource::<PipelineManager>();
    let pipeline =
        |id: Option<CachedPipelineIndex>| match id.map(|id| pipeline_manager.get_pipeline(id)) {
            Some(CachedPipelineStatus::OkRender(p)) => Some(p),
            _ => None
        };
    let (Some(downsample), Some(upsample)) = (
        pipeline(
            world
                .resource::<RenderAssets<BloomDownsamplePipeline>>()
                .iter()
                .next()
                .map(|(_, p)| p.0)
        ),
        pipeline(
            world
                .resource::<RenderAssets<BloomUpsamplePipeline>>()
                .iter()
                .next()
                .map(|(_, p)| p.0)
        )
    ) else {
        return false;
    };
    let Some((_, params)) = world
        .resource::<RenderAssets<GpuRenderBinding<PostProcessParamsBinding>>>()
        .iter()
        .next()
    else {
        return false;
    };
    let meshes = world.resource::<RenderAssets<GpuMesh>>();
    let Some((Some(vertices), Some(indices))) = world
        .resource::<PostProcessingMesh>()
        .0
        .as_ref()
        .and_then(|m| meshes.get(m.id()))
        .map(|m| (m.vertex_buffer.as_ref(), m.index_buffer.as_ref()))
    else {
        return false;
    };
    let gpu_textures = world.resource::<RenderAssets<GpuTexture>>();
    let Some(mips) = bind_groups.textures[1..]
        .iter()
        .map(|id| gpu_textures.get(*id))
        .collect::<Option<Vec<_>>>()
    else {
        return false;
    };

    // Source `i` of the bind groups is mip `i - 1` (source 0 being the render texture)
    let downsamples = (0..mips.len()).map(|mip| (mip, mip, downsample, true));
    let upsamples = (0..mips.len() - 1)
        .rev()
        .map(|mip| (mip, mip + 2, upsample, false));
    for (target, source, pipeline, is_downsample) in downsamples.chain(upsamples) {
        let render_pass = command_buffer.create_render_pass("bloom", |builder| {
            builder.add_color_attachment(RenderPassColorAttachment {
                texture: Some(&mips[target].texture.view),
                load: if is_downsample {
                    LoadOp::Clear(WdeColor::BLACK.into())
                } else {
                    LoadOp::Load
                },
                store: StoreOp::Store,
                resolve_target: None
            });
            Ok(())
        });
        let mut render_pass = match render_pass {
            Ok(render_pass) => render_pass,
            Err(e) => {
                debug!("Failed to create the bloom render pass: {}.", e);
                return false;
            }
        };
        // Before the pipeline, as the raw access resets the render pass state
        if !is_downsample {
            render_pass.raw_mut().set_blend_constant(
                WdeColor::from_linear_rgba(scatter, scatter, scatter, scatter).into()
            );
        }
        if render_pass.set_pipeline(pipeline).is_err() {
            return false;
        }
        render_pass
            .set_bind_group(0, &bind_groups.sources[source])
            .set_bind_group(1, &params.bind_group)
            .set_vertex_buffer(0, vertices)
            .set_index_buffer(indices);
        if is_downsample {
            let first = u32::from(target == 0);
            render_pass.set_push_constants(ShaderStages::FRAGMENT, bytemuck::bytes_of(&first));
        }
        if render_pass.draw_indexed(0..6, 0..1).is_err() {
            return false;
        }
    }
    true
}

#[derive(TypePath, Default, Clone, Debug)]
pub(crate) struct BloomDownsamplePipeline(pub CachedPipelineIndex);
impl RenderAsset for BloomDownsamplePipeline {
    type SourceAsset = RenderPipelineAsset<BloomDownsamplePipeline>;
    type Params = (
        SRes<AssetServer>,
        SResMut<PipelineManager>,
        SBinding<PostProcessParamsBinding>
    );

    fn prepare(
        _id: AssetId<Self::SourceAsset>,
        asset: Self::SourceAsset,
        (assets_server, pipeline_manager, params): &mut SystemParamItem<Self::Params>
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        Ok(BloomDownsamplePipeline(
            pipeline_manager.create_render_pipeline(
                RenderPipelineDescriptor {
                    label: "bloom-downsample",
                    vert: Some(assets_server.load("core/render/post_process/fullscreen.vert.wgsl")),
                    frag: Some(
                        assets_server.load("core/render/post_process/bloom_downsample.frag.wgsl")
                    ),
                    bind_group_layouts: vec![
                        Some(bloom_source_layout()),
                        params.iter().next().map(|(_, p)| p.layout.clone()),
                    ],
                    push_constants: vec![PushConstantDescriptor {
                        stages: ShaderStages::FRAGMENT,
                        offset: 0,
                        size: 4
                    }],
                    depth: DepthDescriptor {
                        enabled: false,
                        ..default()
                    },
                    render_targets: Some(vec![RenderTexture::FORMAT]),
                    ..default()
                },
                asset
            )?
        ))
    }
}

#[derive(TypePath, Default, Clone, Debug)]
pub(crate) struct BloomUpsamplePipeline(pub CachedPipelineIndex);
impl RenderAsset for BloomUpsamplePipeline {
    type SourceAsset = RenderPipelineAsset<BloomUpsamplePipeline>;
    type Params = (
        SRes<AssetServer>,
        SResMut<PipelineManager>,
        SBinding<PostProcessParamsBinding>
    );

    fn prepare(
        _id: AssetId<Self::SourceAsset>,
        asset: Self::SourceAsset,
        (assets_server, pipeline_manager, params): &mut SystemParamItem<Self::Params>
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        // Blends the upsampled mip over the target as `lerp(target, upsampled, scatter)`
        let lerp = BlendComponent {
            src_factor: BlendFactor::Constant,
            dst_factor: BlendFactor::OneMinusConstant,
            operation: BlendOperation::Add
        };
        Ok(BloomUpsamplePipeline(
            pipeline_manager.create_render_pipeline(
                RenderPipelineDescriptor {
                    label: "bloom-upsample",
                    vert: Some(assets_server.load("core/render/post_process/fullscreen.vert.wgsl")),
                    frag: Some(
                        assets_server.load("core/render/post_process/bloom_upsample.frag.wgsl")
                    ),
                    bind_group_layouts: vec![
                        Some(bloom_source_layout()),
                        params.iter().next().map(|(_, p)| p.layout.clone()),
                    ],
                    depth: DepthDescriptor {
                        enabled: false,
                        ..default()
                    },
                    fragment_blend: Some(BlendState {
                        color: lerp,
                        alpha: lerp
                    }),
                    render_targets: Some(vec![RenderTexture::FORMAT]),
                    ..default()
                },
                asset
            )?
        ))
    }
}
