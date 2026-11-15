//! Post-processing of the HDR [`RenderTexture`](crate::textures::RenderTexture), configured by the [`PostProcessSettings`] resource.
//!
//! The effects run in these render passes:
//! - [`RenderPassFog`] (ID 30): exponential height fog, blended over the opaque geometry and the sky, before the transparent objects.
//! - [`RenderPassHdrResolve`] (ID 90): resolves the MSAA render texture into the HDR texture of the [`PostProcessTextures`].
//! - [`RenderPassBloom`] (ID 91): downsamples then upsamples the HDR texture through the bloom mips.
//! - [`RenderPassPostProcess`] (ID 100): composites the chromatic aberration, the bloom, the color grading, the tonemapping,
//!   the vignette, the film grain and the dithering to the swapchain.
//!
//! To add a full-screen effect over the final image, add a sub-pass to [`RenderPassPostProcess`]. To add an HDR effect,
//! add a pass between 90 and 100 that reads and writes the HDR texture of the [`PostProcessTextures`].
use bevy::prelude::*;
use wde_renderer::prelude::*;

mod data;
#[cfg(debug_assertions)]
mod editor;
mod pass_bloom;
mod pass_composite;
mod pass_fog;
mod settings;

pub use data::{BLOOM_MIP_COUNT, PostProcessTextures};
pub use pass_bloom::{RenderPassBloom, RenderPassHdrResolve};
pub use pass_composite::RenderPassPostProcess;
pub use pass_fog::RenderPassFog;
pub use settings::*;

use data::*;
use pass_bloom::*;
use pass_composite::*;
use pass_fog::*;

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
                RenderBindingRegisterPlugin::<FogBinding>::default(),
                RenderPipelineRegisterPlugin::<FogPipeline>::default(),
                RenderPipelineRegisterPlugin::<BloomDownsamplePipeline>::default(),
                RenderPipelineRegisterPlugin::<BloomUpsamplePipeline>::default(),
                RenderPipelineRegisterPlugin::<CompositePipeline>::default()
            ));
        #[cfg(debug_assertions)]
        app.add_plugins(editor::PostProcessEditorPlugin);

        let render_app = app.get_sub_app_mut(RenderApp).unwrap();
        render_app
            .init_resource::<PostProcessUniform>()
            .init_resource::<FogUniform>()
            .init_resource::<BloomBindGroups>()
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
            .add_pass::<RenderPassHdrResolve>()
            .add_pass::<RenderPassBloom>()
            .add_pass::<RenderPassPostProcess>()
            .add_sub_pass::<SubRenderPassComposite, RenderPassPostProcess>();
    }
}
