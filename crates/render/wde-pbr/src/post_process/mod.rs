//! Post-processing of the HDR [`RenderTexture`](crate::textures::RenderTexture), configured by the [`PostProcessSettings`] resource.
//!
//! The effects run in these render passes:
//! - [`RenderPassFog`] (ID 30): exponential height fog, blended over the opaque geometry and the sky, before the transparent objects.
//! - [`RenderPassBloom`] (ID 91): downsamples then upsamples the render texture through the bloom mips.
//! - [`RenderPassExposure`] (ID 92): eye adaptation, from a luminance histogram of the render texture (see [`ExposureSettings`]).
//! - [`RenderPassComposite`] (ID 95): composites the chromatic aberration, the bloom, the exposure, the color grading, the tonemapping,
//!   the vignette, the film grain and the dithering into the LDR texture of the [`PostProcessTextures`].
//! - [`RenderPassSmaa`] (ID 96): edge detection and blending weights of the SMAA, when it is the [`AntiAliasing`].
//! - [`RenderPassPostProcess`] (ID 100): anti-aliases the LDR texture to the swapchain (see [`AntiAliasing`]).
//!
//! To add a full-screen effect over the final image, add a sub-pass to [`RenderPassPostProcess`]. To add an HDR effect,
//! add a pass between 50 and 91 that reads and writes the render texture.
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

pub use data::{BLOOM_MIP_COUNT, PostProcessTextures};
pub use pass_bloom::RenderPassBloom;
pub use pass_composite::{RenderPassComposite, RenderPassPostProcess};
pub use pass_exposure::RenderPassExposure;
pub use pass_fog::RenderPassFog;
pub use settings::*;
pub use smaa::{RenderPassSmaa, SmaaTextures};

use data::*;
use pass_bloom::*;
use pass_composite::*;
use pass_exposure::*;
use pass_fog::*;
use smaa::*;

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
                RenderBindingRegisterPlugin::<FogBinding>::default(),
                RenderPipelineRegisterPlugin::<FogPipeline>::default(),
                RenderPipelineRegisterPlugin::<BloomDownsamplePipeline>::default(),
                RenderPipelineRegisterPlugin::<BloomUpsamplePipeline>::default(),
                RenderPipelineRegisterPlugin::<ExposureHistogramPipeline>::default(),
                RenderPipelineRegisterPlugin::<ExposureAveragePipeline>::default(),
                RenderPipelineRegisterPlugin::<CompositePipeline>::default(),
                RenderPipelineRegisterPlugin::<AntiAliasingPipeline>::default()
            ))
            .add_plugins((
                RenderDataRegisterPlugin::<SmaaTextures>::default(),
                RenderBindingRegisterPlugin::<SmaaEdgesBinding>::default(),
                RenderBindingRegisterPlugin::<SmaaWeightsBinding>::default(),
                RenderBindingRegisterPlugin::<SmaaBlendBinding>::default(),
                RenderPipelineRegisterPlugin::<SmaaEdgesPipeline>::default(),
                RenderPipelineRegisterPlugin::<SmaaWeightsPipeline>::default(),
                RenderPipelineRegisterPlugin::<SmaaBlendPipeline>::default()
            ));
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
            .add_systems(Extract, extract_post_process_params)
            .add_systems(
                Render,
                update_post_process_buffers.in_set(RenderSet::Prepare)
            );
        render_app
            .world_mut()
            .resource_mut::<RenderGraph>()
            .add_pass::<RenderPassFog>()
            .add_sub_pass::<SubRenderPassFog, RenderPassFog>()
            .add_pass::<RenderPassBloom>()
            .add_pass::<RenderPassExposure>()
            .add_pass::<RenderPassComposite>()
            .add_sub_pass::<SubRenderPassComposite, RenderPassComposite>()
            .add_pass::<RenderPassSmaa>()
            .add_pass::<RenderPassPostProcess>()
            .add_sub_pass::<SubRenderPassAntiAliasing, RenderPassPostProcess>();
    }
}
