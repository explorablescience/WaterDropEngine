use wde_logger::prelude::*;

use bevy::{
    ecs::system::{
        SystemParamItem,
        lifetimeless::{SQuery, SRes, SResMut}
    },
    prelude::*
};
use wde_renderer::{prelude::*, wgpu_utils::*};

use super::data::{PostProcessBuffers, PostProcessTextures};
use super::settings::{AntiAliasing, PostProcessUniform};

/// Precomputed lookup textures of the SMAA, from its reference implementation.
#[derive(Resource)]
pub(crate) struct SmaaLuts {
    /// Areas covered on each side of an edge, by distance to its ends and crossing edges.
    area: Handle<Texture>,
    /// Length to add at the end of the edge searches.
    search: Handle<Texture>
}
impl SmaaLuts {
    pub fn load(asset_server: &AssetServer) -> Self {
        let lut = |path: &'static str, label: &'static str, format| {
            asset_server.load_with_settings(path, move |settings: &mut TextureLoaderSettings| {
                settings.label = label.to_string();
                settings.format = format;
            })
        };
        Self {
            area: lut(
                "core/render/post_process/smaa_area_lut.png",
                "smaa-area-lut",
                TextureFormat::Rg8Unorm
            ),
            search: lut(
                "core/render/post_process/smaa_search_lut.png",
                "smaa-search-lut",
                TextureFormat::R8Unorm
            )
        }
    }
}

/// Intermediate textures of the SMAA, resized with the window.
#[derive(Asset, Clone, TypePath, Default)]
pub struct SmaaTextures;
impl SmaaTextures {
    /// Edges with the left (r) and top (g) neighbors.
    pub const EDGES_IDX: u32 = 0;
    /// Blending weights with the left/right (rg) and top/bottom (ba) neighbors.
    pub const WEIGHTS_IDX: u32 = 1;

    pub const EDGES_FORMAT: TextureFormat = TextureFormat::Rg8Unorm;
    pub const WEIGHTS_FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;
}
impl RenderData for SmaaTextures {
    type Params = (SQuery<&'static Window>, SRes<Messages<SurfaceResized>>);

    fn describe((window, _): &mut SystemParamItem<Self::Params>, builder: &mut RenderDataBuilder) {
        let size = {
            let window = window.single().unwrap();
            (
                window.resolution.physical_width().max(1),
                window.resolution.physical_height().max(1)
            )
        };
        for (idx, label, format) in [
            (Self::EDGES_IDX, "smaa-edges", Self::EDGES_FORMAT),
            (Self::WEIGHTS_IDX, "smaa-weights", Self::WEIGHTS_FORMAT)
        ] {
            builder.add_texture(
                idx,
                Texture {
                    label: label.to_string(),
                    size,
                    format,
                    usages: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                    ..Default::default()
                }
            );
        }
    }

    fn recreate((_, surface_resized): &SystemParamItem<Self::Params>) -> Option<bool> {
        Some(
            surface_resized
                .get_cursor()
                .read(surface_resized)
                .next()
                .is_some()
        )
    }
}

/// Inputs of the edge detection: the tonemapped image and the parameters.
#[derive(Asset, Clone, TypePath, Default)]
pub(crate) struct SmaaEdgesBinding;
impl RenderBinding for SmaaEdgesBinding {
    type Params = (
        SRenderData<PostProcessTextures>,
        SRenderData<PostProcessBuffers>
    );

    fn describe(
        &mut self,
        (textures, buffers): &SystemParamItem<Self::Params>,
        builder: &mut RenderBindingBuilder
    ) {
        builder
            .add_texture_view(textures, PostProcessTextures::LDR_IDX)
            .add_texture_sampler(textures, PostProcessTextures::LDR_IDX)
            .add_buffer(buffers, PostProcessBuffers::PARAMS_IDX);
    }

    fn has_dependencies(&self) -> bool {
        true
    }

    fn label(&self) -> &str {
        "smaa-edges-binding"
    }
}

/// Inputs of the blending weights: the edges, the lookup textures and the parameters.
#[derive(Asset, Clone, TypePath, Default)]
pub(crate) struct SmaaWeightsBinding;
impl RenderBinding for SmaaWeightsBinding {
    type Params = (
        SRenderData<SmaaTextures>,
        SRenderData<PostProcessBuffers>,
        SRes<SmaaLuts>
    );

    fn describe(
        &mut self,
        (textures, buffers, luts): &SystemParamItem<Self::Params>,
        builder: &mut RenderBindingBuilder
    ) {
        builder
            .add_texture_view(textures, SmaaTextures::EDGES_IDX)
            .add_texture_sampler(textures, SmaaTextures::EDGES_IDX)
            .add_texture_view_from_id(Some(luts.area.id()))
            .add_texture_view_from_id(Some(luts.search.id()))
            .add_buffer(buffers, PostProcessBuffers::PARAMS_IDX);
    }

    fn has_dependencies(&self) -> bool {
        true
    }

    fn label(&self) -> &str {
        "smaa-weights-binding"
    }
}

/// Inputs of the neighborhood blending: the tonemapped image, the blending weights and the parameters.
#[derive(Asset, Clone, TypePath, Default)]
pub(crate) struct SmaaBlendBinding;
impl RenderBinding for SmaaBlendBinding {
    type Params = (
        SRenderData<PostProcessTextures>,
        SRenderData<SmaaTextures>,
        SRenderData<PostProcessBuffers>
    );

    fn describe(
        &mut self,
        (ldr, smaa, buffers): &SystemParamItem<Self::Params>,
        builder: &mut RenderBindingBuilder
    ) {
        builder
            .add_texture_view(ldr, PostProcessTextures::LDR_IDX)
            .add_texture_sampler(ldr, PostProcessTextures::LDR_IDX)
            .add_texture_view(smaa, SmaaTextures::WEIGHTS_IDX)
            .add_texture_sampler(smaa, SmaaTextures::WEIGHTS_IDX)
            .add_buffer(buffers, PostProcessBuffers::PARAMS_IDX);
    }

    fn has_dependencies(&self) -> bool {
        true
    }

    fn label(&self) -> &str {
        "smaa-blend-binding"
    }
}

/// First two passes of the SMAA, from the LDR texture of the [`PostProcessTextures`]: the edge detection,
/// then the blending weights of each pixel. The neighborhood blending is done by [`RenderPassPostProcess`](super::RenderPassPostProcess).
///
/// It has a render index of 96, and does nothing when the [`AntiAliasing`] is not SMAA.
pub struct RenderPassSmaa;
impl RenderPass for RenderPassSmaa {
    type Params = ();

    fn describe(_: &SystemParamItem<Self::Params>) -> RenderPassDesc {
        RenderPassDesc::default()
    }

    fn id() -> RenderPassId {
        96
    }
    fn label() -> &'static str {
        "smaa"
    }

    fn custom_render(world: &mut World, command_buffer: &mut CommandBuffer) -> Option<bool> {
        if world.resource::<PostProcessUniform>().anti_aliasing() != AntiAliasing::Smaa {
            return Some(true);
        }
        Some(render_smaa(world, command_buffer))
    }
}

fn render_smaa(world: &World, command_buffer: &mut CommandBuffer) -> bool {
    let pipeline_manager = world.resource::<PipelineManager>();
    let pipeline =
        |id: Option<CachedPipelineIndex>| match id.map(|id| pipeline_manager.get_pipeline(id)) {
            Some(CachedPipelineStatus::OkRender(p)) => Some(p),
            _ => None
        };
    let (Some(edges_pipeline), Some(weights_pipeline)) = (
        pipeline(
            world
                .resource::<RenderAssets<SmaaEdgesPipeline>>()
                .iter()
                .next()
                .map(|(_, p)| p.0)
        ),
        pipeline(
            world
                .resource::<RenderAssets<SmaaWeightsPipeline>>()
                .iter()
                .next()
                .map(|(_, p)| p.0)
        )
    ) else {
        return false;
    };

    fn bind_group<B: RenderBinding + Asset + Clone>(world: &World) -> Option<&BindGroup> {
        world
            .resource::<RenderAssets<GpuRenderBinding<B>>>()
            .iter()
            .next()
            .map(|(_, b)| &b.bind_group)
    }
    let (Some(edges_input), Some(weights_input)) = (
        bind_group::<SmaaEdgesBinding>(world),
        bind_group::<SmaaWeightsBinding>(world)
    ) else {
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
    let Some((_, data)) = world
        .resource::<RenderAssets<GpuRenderData<SmaaTextures>>>()
        .iter()
        .next()
    else {
        return false;
    };
    let texture = |idx| data.get_texture(idx).and_then(|t| gpu_textures.get(t.id()));
    let (Some(edges), Some(weights)) = (
        texture(SmaaTextures::EDGES_IDX),
        texture(SmaaTextures::WEIGHTS_IDX)
    ) else {
        return false;
    };

    // Debug views: the weights step also shows its inputs
    let id = |idx| data.get_texture(idx).unwrap().id();
    let (edges_id, weights_id) = (id(SmaaTextures::EDGES_IDX), id(SmaaTextures::WEIGHTS_IDX));
    let luts = world.resource::<SmaaLuts>();
    let edges_step = [edges_id];
    let weights_step = [weights_id, edges_id, luts.area.id(), luts.search.id()];

    // Both targets are cleared, as the edge detection discards the pixels without edges
    for (target, pipeline, bind_group, step, debug_textures) in [
        (edges, edges_pipeline, edges_input, "edges", &edges_step[..]),
        (
            weights,
            weights_pipeline,
            weights_input,
            "weights",
            &weights_step[..]
        )
    ] {
        let mut render_pass = match command_buffer.create_render_pass("smaa", |builder| {
            builder.add_color_attachment(RenderPassColorAttachment {
                texture: Some(&target.texture.view),
                load: LoadOp::Clear(WdeColor::TRANSPARENT.into()),
                store: StoreOp::Store,
                resolve_target: None
            });
            Ok(())
        }) {
            Ok(render_pass) => render_pass,
            Err(e) => {
                trace!("Failed to create the SMAA render pass: {}.", e);
                return false;
            }
        };
        if render_pass.set_pipeline(pipeline).is_err() {
            return false;
        }
        render_pass.set_bind_group(0, bind_group);
        render_pass
            .set_vertex_buffer(0, vertices)
            .set_index_buffer(indices);
        if render_pass.draw_indexed(0..6, 0..1).is_err() {
            return false;
        }
        drop(render_pass);
        RenderGraphDebug::capture(world, command_buffer, step, debug_textures);
    }
    true
}

fn fullscreen_pipeline(
    label: &'static str,
    frag: &'static str,
    layout: Option<BindGroupLayout>,
    render_targets: Option<Vec<TextureFormat>>,
    asset_server: &AssetServer
) -> RenderPipelineDescriptor {
    RenderPipelineDescriptor {
        label,
        vert: Some(asset_server.load("core/render/post_process/fullscreen.vert.wgsl")),
        frag: Some(asset_server.load(frag)),
        bind_group_layouts: vec![layout],
        depth: DepthDescriptor {
            enabled: false,
            ..default()
        },
        render_targets,
        ..default()
    }
}

#[derive(TypePath, Default, Clone, Debug)]
pub(crate) struct SmaaEdgesPipeline(pub CachedPipelineIndex);
impl RenderAsset for SmaaEdgesPipeline {
    type SourceAsset = RenderPipelineAsset<SmaaEdgesPipeline>;
    type Params = (
        SRes<AssetServer>,
        SResMut<PipelineManager>,
        SBinding<SmaaEdgesBinding>
    );

    fn prepare(
        _id: AssetId<Self::SourceAsset>,
        asset: Self::SourceAsset,
        (asset_server, pipeline_manager, binding): &mut SystemParamItem<Self::Params>
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        Ok(SmaaEdgesPipeline(pipeline_manager.create_render_pipeline(
            fullscreen_pipeline(
                "smaa-edges",
                "core/render/post_process/smaa_edges.frag.wgsl",
                binding.iter().next().map(|(_, b)| b.layout.clone()),
                Some(vec![SmaaTextures::EDGES_FORMAT]),
                asset_server
            ),
            asset
        )?))
    }
}

#[derive(TypePath, Default, Clone, Debug)]
pub(crate) struct SmaaWeightsPipeline(pub CachedPipelineIndex);
impl RenderAsset for SmaaWeightsPipeline {
    type SourceAsset = RenderPipelineAsset<SmaaWeightsPipeline>;
    type Params = (
        SRes<AssetServer>,
        SResMut<PipelineManager>,
        SBinding<SmaaWeightsBinding>
    );

    fn prepare(
        _id: AssetId<Self::SourceAsset>,
        asset: Self::SourceAsset,
        (asset_server, pipeline_manager, binding): &mut SystemParamItem<Self::Params>
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        Ok(SmaaWeightsPipeline(
            pipeline_manager.create_render_pipeline(
                fullscreen_pipeline(
                    "smaa-weights",
                    "core/render/post_process/smaa_weights.frag.wgsl",
                    binding.iter().next().map(|(_, b)| b.layout.clone()),
                    Some(vec![SmaaTextures::WEIGHTS_FORMAT]),
                    asset_server
                ),
                asset
            )?
        ))
    }
}

/// Neighborhood blending, drawn to the swapchain by the anti-aliasing sub-pass.
#[derive(TypePath, Default, Clone, Debug)]
pub(crate) struct SmaaBlendPipeline(pub CachedPipelineIndex);
impl RenderAsset for SmaaBlendPipeline {
    type SourceAsset = RenderPipelineAsset<SmaaBlendPipeline>;
    type Params = (
        SRes<AssetServer>,
        SResMut<PipelineManager>,
        SBinding<SmaaBlendBinding>
    );

    fn prepare(
        _id: AssetId<Self::SourceAsset>,
        asset: Self::SourceAsset,
        (asset_server, pipeline_manager, binding): &mut SystemParamItem<Self::Params>
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        Ok(SmaaBlendPipeline(pipeline_manager.create_render_pipeline(
            fullscreen_pipeline(
                "smaa-blend",
                "core/render/post_process/smaa_blend.frag.wgsl",
                binding.iter().next().map(|(_, b)| b.layout.clone()),
                None,
                asset_server
            ),
            asset
        )?))
    }
}
