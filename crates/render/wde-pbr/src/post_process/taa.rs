use wde_logger::prelude::*;

use bevy::{
    ecs::system::{
        SystemParamItem,
        lifetimeless::{SQuery, SRes, SResMut}
    },
    prelude::*
};
use wde_camera::prelude::*;
use wde_renderer::{prelude::*, wgpu_utils::*};

use super::data::PostProcessBuffers;
use super::settings::{AntiAliasing, PostProcessSettings, PostProcessUniform};
use crate::textures::{DeferredTextures, RenderTexture};

/// Textures of the TAA, resized with the window. They are in the [`RenderTexture::FORMAT`].
#[derive(Asset, Clone, TypePath, Default)]
pub struct TaaTextures;
impl TaaTextures {
    /// The anti-aliased image of this frame, copied to the history and to the render texture.
    pub const RESOLVED_IDX: u32 = 0;
    /// The anti-aliased image of the previous frame. Its alpha is 0 until it holds a frame.
    pub const HISTORY_IDX: u32 = 1;
}
impl RenderData for TaaTextures {
    type Params = (SQuery<&'static Window>, SRes<Messages<SurfaceResized>>);

    fn describe((window, _): &mut SystemParamItem<Self::Params>, builder: &mut RenderDataBuilder) {
        let size = {
            let window = window.single().unwrap();
            (
                window.resolution.physical_width().max(1),
                window.resolution.physical_height().max(1)
            )
        };
        for (idx, label) in [
            (Self::RESOLVED_IDX, "taa-resolved"),
            (Self::HISTORY_IDX, "taa-history")
        ] {
            builder.add_texture(
                idx,
                Texture {
                    label: label.to_string(),
                    size,
                    format: RenderTexture::FORMAT,
                    usages: TextureUsages::RENDER_ATTACHMENT
                        | TextureUsages::TEXTURE_BINDING
                        | TextureUsages::COPY_SRC,
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

/// Inputs of the resolve: the current frame, its linear depth, the history and the parameters.
#[derive(Asset, Clone, TypePath, Default)]
pub(crate) struct TaaBinding;
impl RenderBinding for TaaBinding {
    type Params = (
        SRenderData<RenderTexture>,
        SRenderData<DeferredTextures>,
        SRenderData<TaaTextures>,
        SRenderData<PostProcessBuffers>
    );

    fn describe(
        &mut self,
        (render_texture, deferred, taa, buffers): &SystemParamItem<Self::Params>,
        builder: &mut RenderBindingBuilder
    ) {
        builder
            .add_texture_view(render_texture, RenderTexture::BINDING)
            .add_texture_view(deferred, DeferredTextures::DEPTH_IDX)
            .add_texture_view(taa, TaaTextures::HISTORY_IDX)
            .add_texture_sampler(taa, TaaTextures::HISTORY_IDX)
            .add_buffer(buffers, PostProcessBuffers::PARAMS_IDX);
    }

    fn has_dependencies(&self) -> bool {
        true
    }

    fn label(&self) -> &str {
        "taa-binding"
    }
}

/// Whether the TAA ran last frame: its history is cleared when it starts again.
#[derive(Resource, Default)]
pub(crate) struct TaaState {
    active: bool
}

/// Jitters the camera projection while the TAA is selected.
pub(crate) fn sync_projection_jitter(
    settings: Res<PostProcessSettings>,
    mut jitter: ResMut<ProjectionJitter>
) {
    let enabled = settings.anti_aliasing == AntiAliasing::Taa;
    if jitter.enabled != enabled {
        jitter.enabled = enabled;
    }
}

/// Temporal anti-aliasing of the HDR [`RenderTexture`], before the bloom.
///
/// It blends the frame with the history reprojected by the camera motion, clipped to the colors around each pixel,
/// into the resolved texture of the [`TaaTextures`], then copies it to the history and back to the render texture.
///
/// It has a render index of 90, and does nothing when the [`AntiAliasing`] is not TAA.
pub struct RenderPassTaa;
impl RenderPass for RenderPassTaa {
    type Params = ();

    fn describe(_: &SystemParamItem<Self::Params>) -> RenderPassDesc {
        RenderPassDesc::default()
    }

    fn id() -> RenderPassId {
        90
    }
    fn label() -> &'static str {
        "taa"
    }

    fn custom_render(world: &mut World, command_buffer: &mut CommandBuffer) -> Option<bool> {
        if world.resource::<PostProcessUniform>().anti_aliasing() != AntiAliasing::Taa {
            world.resource_mut::<TaaState>().active = false;
            return Some(true);
        }
        let rendered = render_taa(world, command_buffer);
        world.resource_mut::<TaaState>().active = rendered;
        Some(rendered)
    }
}

fn render_taa(world: &World, command_buffer: &mut CommandBuffer) -> bool {
    let Some(CachedPipelineStatus::OkRender(pipeline)) = world
        .resource::<RenderAssets<TaaPipeline>>()
        .iter()
        .next()
        .map(|(_, p)| world.resource::<PipelineManager>().get_pipeline(p.0))
    else {
        return false;
    };

    fn bind_group<B: RenderBinding + Asset + Clone>(world: &World) -> Option<&BindGroup> {
        world
            .resource::<RenderAssets<GpuRenderBinding<B>>>()
            .iter()
            .next()
            .map(|(_, b)| &b.bind_group)
    }
    let (Some(camera), Some(inputs)) = (
        bind_group::<CameraBinding>(world),
        bind_group::<TaaBinding>(world)
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
    let (Some((_, taa)), Some((_, render))) = (
        world
            .resource::<RenderAssets<GpuRenderData<TaaTextures>>>()
            .iter()
            .next(),
        world
            .resource::<RenderAssets<GpuRenderData<RenderTexture>>>()
            .iter()
            .next()
    ) else {
        return false;
    };
    let (Some(resolved_id), Some(history_id), Some(render_id)) = (
        taa.get_texture(TaaTextures::RESOLVED_IDX).map(|t| t.id()),
        taa.get_texture(TaaTextures::HISTORY_IDX).map(|t| t.id()),
        render.get_texture(RenderTexture::BINDING).map(|t| t.id())
    ) else {
        return false;
    };
    let (Some(resolved), Some(history), Some(render)) = (
        gpu_textures.get(resolved_id),
        gpu_textures.get(history_id),
        gpu_textures.get(render_id)
    ) else {
        return false;
    };

    // A stale history would be blended in where the camera has not moved since
    if !world.resource::<TaaState>().active {
        let clear = command_buffer.create_render_pass("taa-clear-history", |builder| {
            builder.add_color_attachment(RenderPassColorAttachment {
                texture: Some(&history.texture.view),
                load: LoadOp::Clear(WdeColor::TRANSPARENT.into()),
                store: StoreOp::Store,
                resolve_target: None
            });
            Ok(())
        });
        if let Err(e) = clear {
            trace!("Failed to clear the TAA history: {}.", e);
            return false;
        }
    }

    let mut render_pass = match command_buffer.create_render_pass("taa", |builder| {
        builder.add_color_attachment(RenderPassColorAttachment {
            texture: Some(&resolved.texture.view),
            load: LoadOp::Clear(WdeColor::TRANSPARENT.into()),
            store: StoreOp::Store,
            resolve_target: None
        });
        Ok(())
    }) {
        Ok(render_pass) => render_pass,
        Err(e) => {
            trace!("Failed to create the TAA render pass: {}.", e);
            return false;
        }
    };
    if render_pass.set_pipeline(pipeline).is_err() {
        return false;
    }
    render_pass.set_bind_group(0, camera);
    render_pass.set_bind_group(1, inputs);
    render_pass
        .set_vertex_buffer(0, vertices)
        .set_index_buffer(indices);
    if render_pass.draw_indexed(0..6, 0..1).is_err() {
        return false;
    }
    drop(render_pass);

    command_buffer.copy_texture_to_texture(&resolved.texture.texture, &history.texture.texture);
    command_buffer.copy_texture_to_texture(&resolved.texture.texture, &render.texture.texture);
    RenderGraphDebug::capture(world, command_buffer, "resolve", &[resolved_id]);
    true
}

#[derive(TypePath, Default, Clone, Debug)]
pub(crate) struct TaaPipeline(pub CachedPipelineIndex);
impl RenderAsset for TaaPipeline {
    type SourceAsset = RenderPipelineAsset<TaaPipeline>;
    type Params = (
        SRes<AssetServer>,
        SResMut<PipelineManager>,
        SBinding<CameraBinding>,
        SBinding<TaaBinding>
    );

    fn prepare(
        _id: AssetId<Self::SourceAsset>,
        asset: Self::SourceAsset,
        (asset_server, pipeline_manager, camera, binding): &mut SystemParamItem<Self::Params>
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        Ok(TaaPipeline(pipeline_manager.create_render_pipeline(
            RenderPipelineDescriptor {
                label: "taa",
                vert: Some(asset_server.load("core/render/post_process/fullscreen.vert.wgsl")),
                frag: Some(asset_server.load("core/render/post_process/taa.frag.wgsl")),
                bind_group_layouts: vec![
                    camera.iter().next().map(|(_, c)| c.layout.clone()),
                    binding.iter().next().map(|(_, b)| b.layout.clone()),
                ],
                depth: DepthDescriptor {
                    enabled: false,
                    ..default()
                },
                render_targets: Some(vec![RenderTexture::FORMAT]),
                ..default()
            },
            asset
        )?))
    }
}
