//! Post-processing of the HDR [`RenderTexture`](crate::textures::RenderTexture), configured by the [`PostProcessSettings`] resource.
//!
//! The effects run in these render passes:
//! - [`RenderPassFog`] (ID 30): exponential height fog, blended over the opaque geometry and the sky, before the transparent objects.
//! - [`RenderPassTaa`] (ID 90): temporal anti-aliasing of the HDR render texture, when it is the [`AntiAliasing`].
//! - [`RenderPassBloom`] (ID 91): downsamples then upsamples the render texture through the bloom mips.
//! - [`RenderPassExposure`] (ID 92): eye adaptation, from a luminance histogram of the render texture (see [`ExposureSettings`]).
//! - [`RenderPassComposite`] (ID 95): composites the chromatic aberration, the bloom, the exposure, the color grading, the tonemapping,
//!   the vignette, the film grain and the dithering into the LDR texture of the [`PostProcessTextures`].
//! - [`RenderPassSmaa`] (ID 96): edge detection and blending weights of the SMAA, when it is the [`AntiAliasing`].
//! - [`RenderPassAntiAliasing`] (ID 97): anti-aliases the LDR texture (see [`AntiAliasing`]).
//! - [`RenderPassPostProcess`] (ID 100): copies the anti-aliased image to the swapchain, sharpening it (see [`SharpeningSettings`]).
//!
//! To add a full-screen effect over the final image, add a sub-pass to [`RenderPassPostProcess`]. To add an HDR effect,
//! add a pass between 50 and 90 that reads and writes the render texture.
use bevy::prelude::*;
use wde_renderer::prelude::*;

mod data;
#[cfg(debug_assertions)]
mod editor;
mod pass_bloom;
mod pass_composite;
mod pass_exposure;
mod pass_fog;
mod settings;
mod smaa;
mod taa;

pub use data::{BLOOM_MIP_COUNT, PostProcessTextures};
pub use pass_bloom::RenderPassBloom;
pub use pass_composite::{RenderPassAntiAliasing, RenderPassComposite, RenderPassPostProcess};
pub use pass_exposure::RenderPassExposure;
pub use pass_fog::RenderPassFog;
pub use settings::*;
pub use smaa::{RenderPassSmaa, SmaaTextures};
pub use taa::{RenderPassTaa, TaaTextures};

use data::*;
use pass_bloom::*;
use pass_composite::*;
use pass_exposure::*;
use pass_fog::*;
use smaa::*;
use taa::*;

pub(crate) struct PostProcessPlugin;
impl Plugin for PostProcessPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PostProcessSettings>()
            .register_type::<PostProcessSettings>()
            .add_plugins((
                RenderDataRegisterPlugin::<PostProcessTextures>::default(),
                RenderDataRegisterPlugin::<PostProcessBuffers>::default(),
                RenderBindingRegisterPlugin::<PostProcessParamsBinding>::default(),
                RenderBindingRegisterPlugin::<PostProcessCompositeBinding>::default(),
                RenderBindingRegisterPlugin::<AntiAliasingBinding>::default(),
                RenderBindingRegisterPlugin::<SharpeningBinding>::default(),
                RenderBindingRegisterPlugin::<FogBinding>::default(),
                RenderPipelineRegisterPlugin::<FogPipeline>::default(),
                RenderPipelineRegisterPlugin::<BloomDownsamplePipeline>::default(),
                RenderPipelineRegisterPlugin::<BloomUpsamplePipeline>::default(),
                RenderPipelineRegisterPlugin::<ExposureHistogramPipeline>::default(),
                RenderPipelineRegisterPlugin::<ExposureAveragePipeline>::default(),
                RenderPipelineRegisterPlugin::<CompositePipeline>::default(),
                RenderPipelineRegisterPlugin::<AntiAliasingPipeline>::default(),
                RenderPipelineRegisterPlugin::<SharpeningPipeline>::default()
            ))
            .add_plugins((
                RenderDataRegisterPlugin::<SmaaTextures>::default(),
                RenderBindingRegisterPlugin::<SmaaEdgesBinding>::default(),
                RenderBindingRegisterPlugin::<SmaaWeightsBinding>::default(),
                RenderBindingRegisterPlugin::<SmaaBlendBinding>::default(),
                RenderPipelineRegisterPlugin::<SmaaEdgesPipeline>::default(),
                RenderPipelineRegisterPlugin::<SmaaWeightsPipeline>::default(),
                RenderPipelineRegisterPlugin::<SmaaBlendPipeline>::default(),
                RenderDataRegisterPlugin::<TaaTextures>::default(),
                RenderBindingRegisterPlugin::<TaaBinding>::default(),
                RenderPipelineRegisterPlugin::<TaaPipeline>::default()
            ))
            .add_systems(PostUpdate, sync_projection_jitter);
        #[cfg(debug_assertions)]
        app.add_plugins(editor::PostProcessEditorPlugin);

        let smaa_luts = SmaaLuts::load(app.world().resource::<AssetServer>());
        let render_app = app.get_sub_app_mut(RenderApp).unwrap();
        render_app
            .insert_resource(smaa_luts)
            .init_resource::<PostProcessUniform>()
            .init_resource::<FogUniform>()
            .init_resource::<BloomBindGroups>()
            .init_resource::<ExposureBindGroup>()
            .init_resource::<TaaState>()
            .add_systems(ExtractSchedule, extract_post_process_params)
            .add_systems(
                Render,
                update_post_process_buffers.in_set(RenderSet::Prepare)
            );
        render_app
            .world_mut()
            .resource_mut::<RenderGraph>()
            .add_pass::<RenderPassFog>()
            .add_sub_pass::<SubRenderPassFog, RenderPassFog>()
            .add_pass::<RenderPassTaa>()
            .add_pass::<RenderPassBloom>()
            .add_pass::<RenderPassExposure>()
            .add_pass::<RenderPassComposite>()
            .add_sub_pass::<SubRenderPassComposite, RenderPassComposite>()
            .add_pass::<RenderPassSmaa>()
            .add_pass::<RenderPassAntiAliasing>()
            .add_sub_pass::<SubRenderPassAntiAliasing, RenderPassAntiAliasing>()
            .add_pass::<RenderPassPostProcess>()
            .add_sub_pass::<SubRenderPassSharpening, RenderPassPostProcess>();
    }
}
