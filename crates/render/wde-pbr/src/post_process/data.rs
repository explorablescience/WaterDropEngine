use bevy::{
    ecs::system::{
        SystemParamItem,
        lifetimeless::{SQuery, SRes}
    },
    prelude::*
};
use wde_renderer::prelude::*;

use crate::prelude::*;

/// Number of bins of the luminance histogram of the eye adaptation.
pub(crate) const EXPOSURE_HISTOGRAM_BINS: usize = 64;

/// Number of bloom mips, each one half the size of the previous one (the first is half the screen size).
pub const BLOOM_MIP_COUNT: u32 = 6;

/// Single-sampled textures of the post-processing stack, resized with the window.
#[derive(Asset, Clone, TypePath, Default)]
pub struct PostProcessTextures;
impl PostProcessTextures {
    /// The tonemapped image, before the anti-aliasing.
    pub const LDR_IDX: u32 = 0;
    /// The anti-aliased image, before the sharpening.
    pub const ANTI_ALIASED_IDX: u32 = 1;
    /// The first bloom mip. Mip `i` is at index `BLOOM_IDX + i`.
    pub const BLOOM_IDX: u32 = 2;

    pub const LDR_FORMAT: TextureFormat = TextureFormat::Rgba8UnormSrgb;
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
        for (idx, label) in [
            (Self::LDR_IDX, "post-process-ldr"),
            (Self::ANTI_ALIASED_IDX, "post-process-anti-aliased")
        ] {
            builder.add_texture(
                idx,
                Texture {
                    label: label.to_string(),
                    size,
                    format: Self::LDR_FORMAT,
                    usages,
                    ..Default::default()
                }
            );
        }
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

/// Buffers of the post-processing stack: the uniforms written each frame, and the eye adaptation state kept on the GPU.
#[derive(Asset, Clone, TypePath, Default)]
pub(crate) struct PostProcessBuffers;
impl PostProcessBuffers {
    pub const PARAMS_IDX: u32 = 0;
    pub const FOG_IDX: u32 = 1;
    /// Luminance histogram of the current frame (`EXPOSURE_HISTOGRAM_BINS` u32 counts).
    pub const HISTOGRAM_IDX: u32 = 2;
    /// Adapted exposure: an f32 in EV stops, then a u32 set once it is initialized.
    pub const EXPOSURE_IDX: u32 = 3;
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
            )
            .add_buffer(
                Self::HISTOGRAM_IDX,
                Buffer {
                    label: "post-process-exposure-histogram".to_string(),
                    size: EXPOSURE_HISTOGRAM_BINS * std::mem::size_of::<u32>(),
                    usage: BufferUsage::STORAGE,
                    content: None
                }
            )
            .add_buffer(
                Self::EXPOSURE_IDX,
                Buffer {
                    label: "post-process-exposure".to_string(),
                    size: 2 * std::mem::size_of::<u32>(),
                    usage: BufferUsage::STORAGE,
                    content: None
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

/// Inputs of the final composite: the render texture, the bloom, the parameters and the adapted exposure.
#[derive(Asset, Clone, TypePath, Default)]
pub(crate) struct PostProcessCompositeBinding;
impl RenderBinding for PostProcessCompositeBinding {
    type Params = (
        SRenderData<RenderTexture>,
        SRenderData<PostProcessTextures>,
        SRenderData<PostProcessBuffers>
    );

    fn describe(
        &mut self,
        (render_texture, textures, buffers): &SystemParamItem<Self::Params>,
        builder: &mut RenderBindingBuilder
    ) {
        builder
            .add_texture_view(render_texture, RenderTexture::BINDING)
            .add_texture_sampler(render_texture, RenderTexture::BINDING)
            .add_texture_view(textures, PostProcessTextures::BLOOM_IDX)
            .add_texture_sampler(textures, PostProcessTextures::BLOOM_IDX)
            .add_buffer(buffers, PostProcessBuffers::PARAMS_IDX)
            .add_buffer(buffers, PostProcessBuffers::EXPOSURE_IDX);
    }

    fn has_dependencies(&self) -> bool {
        true
    }

    fn label(&self) -> &str {
        "post-process-composite-binding"
    }
}

/// Inputs of the anti-aliasing: the tonemapped image and the parameters.
#[derive(Asset, Clone, TypePath, Default)]
pub(crate) struct AntiAliasingBinding;
impl RenderBinding for AntiAliasingBinding {
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
        "post-process-anti-aliasing-binding"
    }
}

/// Inputs of the sharpening: the anti-aliased image and the parameters.
#[derive(Asset, Clone, TypePath, Default)]
pub(crate) struct SharpeningBinding;
impl RenderBinding for SharpeningBinding {
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
            .add_texture_view(textures, PostProcessTextures::ANTI_ALIASED_IDX)
            .add_buffer(buffers, PostProcessBuffers::PARAMS_IDX);
    }

    fn has_dependencies(&self) -> bool {
        true
    }

    fn label(&self) -> &str {
        "post-process-sharpening-binding"
    }
}

/// Inputs of the fog: the linear depth and the fog parameters.
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
            .add_texture_view(deferred, DeferredTextures::DEPTH_IDX)
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
    settings: Extract<Res<PostProcessSettings>>,
    pbr: Extract<Res<PbrSettings>>,
    time: Extract<Res<Time>>,
    #[cfg(feature = "atmosphere")] atmosphere: Extract<Res<AtmosphereSettings>>,
    mut params: ResMut<PostProcessUniform>,
    mut fog: ResMut<FogUniform>
) {
    #[cfg(feature = "atmosphere")]
    let day = atmosphere.day_amount(
        AtmosphereSettings::sun_direction(atmosphere.current_time_of_day(time.elapsed_secs())).y
    );
    #[cfg(not(feature = "atmosphere"))]
    let day = 1.0;
    let settings = settings.for_debug_view(pbr.debug_view);
    *params = PostProcessUniform::from_settings(
        &settings,
        time.elapsed_secs_wrapped(),
        time.delta_secs(),
        day
    );
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
