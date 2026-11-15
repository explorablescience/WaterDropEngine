use bevy::{
    ecs::system::{
        SystemParamItem,
        lifetimeless::{SQuery, SRes}
    },
    prelude::*
};
use wde_renderer::prelude::*;

use crate::prelude::*;

/// Number of bloom mips, each one half the size of the previous one (the first is half the screen size).
pub const BLOOM_MIP_COUNT: u32 = 6;

/// Single-sampled textures of the post-processing stack, resized with the window.
#[derive(Asset, Clone, TypePath, Default)]
pub struct PostProcessTextures;
impl PostProcessTextures {
    /// The resolved HDR render texture.
    pub const HDR_IDX: u32 = 0;
    /// The first bloom mip. Mip `i` is at index `BLOOM_IDX + i`.
    pub const BLOOM_IDX: u32 = 1;
}
impl RenderData for PostProcessTextures {
    type Params = (SQuery<&'static Window>, SRes<Messages<SurfaceResized>>);

    fn describe((window, _): &mut SystemParamItem<Self::Params>, builder: &mut RenderDataBuilder) {
        let size = {
            let window = window.single().unwrap();
            (
                window.resolution.physical_width().max(1),
                window.resolution.physical_height().max(1)
            )
        };
        let usages = TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING;
        builder.add_texture(
            Self::HDR_IDX,
            Texture {
                label: "post-process-hdr".to_string(),
                size,
                format: RenderTexture::FORMAT,
                usages,
                ..Default::default()
            }
        );
        for mip in 0..BLOOM_MIP_COUNT {
            builder.add_texture(
                Self::BLOOM_IDX + mip,
                Texture {
                    label: format!("post-process-bloom-{mip}"),
                    size: ((size.0 >> (mip + 1)).max(1), (size.1 >> (mip + 1)).max(1)),
                    format: RenderTexture::FORMAT,
                    usages,
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

/// Uniform buffers of the post-processing stack, written each frame.
#[derive(Asset, Clone, TypePath, Default)]
pub(crate) struct PostProcessBuffers;
impl PostProcessBuffers {
    pub const PARAMS_IDX: u32 = 0;
    pub const FOG_IDX: u32 = 1;
}
impl RenderData for PostProcessBuffers {
    type Params = ();

    fn describe(_: &mut SystemParamItem<Self::Params>, builder: &mut RenderDataBuilder) {
        builder
            .add_buffer(
                Self::PARAMS_IDX,
                Buffer {
                    label: "post-process-params".to_string(),
                    size: std::mem::size_of::<PostProcessUniform>(),
                    usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST,
                    content: Some(bytemuck::cast_slice(&[PostProcessUniform::default()]).to_vec())
                }
            )
            .add_buffer(
                Self::FOG_IDX,
                Buffer {
                    label: "post-process-fog".to_string(),
                    size: std::mem::size_of::<FogUniform>(),
                    usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST,
                    content: Some(bytemuck::cast_slice(&[FogUniform::default()]).to_vec())
                }
            );
    }
}

/// The post-processing parameters, read by the bloom passes.
#[derive(Asset, Clone, TypePath, Default)]
pub(crate) struct PostProcessParamsBinding;
impl RenderBinding for PostProcessParamsBinding {
    type Params = SRenderData<PostProcessBuffers>;

    fn describe(
        &mut self,
        buffers: &SystemParamItem<Self::Params>,
        builder: &mut RenderBindingBuilder
    ) {
        builder.add_buffer(buffers, PostProcessBuffers::PARAMS_IDX);
    }

    fn label(&self) -> &str {
        "post-process-params-binding"
    }
}

/// Inputs of the final composite: the resolved HDR texture, the bloom and the parameters.
#[derive(Asset, Clone, TypePath, Default)]
pub(crate) struct PostProcessCompositeBinding;
impl RenderBinding for PostProcessCompositeBinding {
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
            .add_texture_view(textures, PostProcessTextures::HDR_IDX)
            .add_texture_sampler(textures, PostProcessTextures::HDR_IDX)
            .add_texture_view(textures, PostProcessTextures::BLOOM_IDX)
            .add_texture_sampler(textures, PostProcessTextures::BLOOM_IDX)
            .add_buffer(buffers, PostProcessBuffers::PARAMS_IDX);
    }

    fn has_dependencies(&self) -> bool {
        true
    }

    fn label(&self) -> &str {
        "post-process-composite-binding"
    }
}

/// Inputs of the fog: the resolved linear depth and the fog parameters.
#[derive(Asset, Clone, TypePath, Default)]
pub(crate) struct FogBinding;
impl RenderBinding for FogBinding {
    type Params = (
        SRenderData<DeferredTextures>,
        SRenderData<PostProcessBuffers>
    );

    fn describe(
        &mut self,
        (deferred, buffers): &SystemParamItem<Self::Params>,
        builder: &mut RenderBindingBuilder
    ) {
        builder
            .add_texture_view(deferred, DeferredTextures::DEPTH_RESOLVED_IDX)
            .add_texture_sampler(deferred, DeferredTextures::DEPTH_RESOLVED_IDX)
            .add_buffer(buffers, PostProcessBuffers::FOG_IDX);
    }

    fn has_dependencies(&self) -> bool {
        true
    }

    fn label(&self) -> &str {
        "post-process-fog-binding"
    }
}

pub(crate) fn extract_post_process_params(
    settings: ExtractWorld<Res<PostProcessSettings>>,
    time: ExtractWorld<Res<Time>>,
    mut params: ResMut<PostProcessUniform>,
    mut fog: ResMut<FogUniform>
) {
    *params = PostProcessUniform::from_settings(&settings, time.elapsed_secs_wrapped());
    *fog = FogUniform::from_settings(&settings.fog);
}

pub(crate) fn update_post_process_buffers(
    data: ResRenderData<PostProcessBuffers>,
    buffers: Res<RenderAssets<GpuBuffer>>,
    render_instance: Res<RenderInstance>,
    params: Res<PostProcessUniform>,
    fog: Res<FogUniform>
) {
    let Some((_, data)) = data.iter().next() else {
        return;
    };
    let render_instance = render_instance.0.read().unwrap();
    let write = |idx: u32, bytes: &[u8]| {
        if let Some(buffer) = data.get_buffer(idx).and_then(|b| buffers.get(&b)) {
            buffer.buffer.write(&render_instance, bytes, 0);
        }
    };
    write(
        PostProcessBuffers::PARAMS_IDX,
        bytemuck::cast_slice(&[*params])
    );
    write(PostProcessBuffers::FOG_IDX, bytemuck::cast_slice(&[*fog]));
}
