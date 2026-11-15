//! Physically based rendering (PBR) integration for WaterDropEngine.
//!
//! This crate wires the PBR rendering stack into the engine and exposes the core
//! building blocks to configure deferred PBR rendering.
//!
//! The PBR rendering stack is built on top of the deferred rendering pipeline, and consists of the following passes:
//! - [`RenderPassEnvironment`] (feature `atmosphere`): Bakes the ambient lighting of the
//!   [`EnvironmentData`] from the sky when the sun moves: spherical harmonics for the diffuse
//!   light, and a prefiltered cube map for the reflections.
//! - [`RenderPassDeferredGBuffer`](crate::passes::RenderPassDeferredGBuffer): Renders the scene geometry into the G-buffer (only opaque objects). It writes to the [`DepthTexture`](crate::textures::DepthTexture) and the G-buffer [`DeferredTextures`](crate::textures::DeferredTextures) (depth, albedo, normal, AO, emissive). Nothing is multisampled: averaging G-buffer samples would mix objects with the background at their edges.
//! - [`RenderPassAmbientOcclusion`](crate::passes::RenderPassAmbientOcclusion): GTAO at half resolution from the G-buffer depth, then a depth-aware blur, configured by the [`AmbientOcclusionSettings`](crate::passes::AmbientOcclusionSettings) resource (disabled by default).
//! - [`RenderPassDeferredLighting`](crate::passes::RenderPassDeferredLighting): Performs the lighting pass, reading from the G-buffer textures, the ambient occlusion (applied to the ambient light only), the cascaded shadow maps and the environment lighting, and writing to the HDR [`RenderTexture`](crate::textures::RenderTexture).
//! - [`RenderPassTransparent`](crate::passes::RenderPassTransparent): Renders transparent objects, using the [`DepthTexture`](crate::textures::DepthTexture) for depth testing, and writing to the same [`RenderTexture`](crate::textures::RenderTexture) as the lighting pass.
//! - The [`post_process`](crate::post_process) passes: fog (before the transparent objects), then the bloom, the composite (color grading, tonemapping, ...), and the anti-aliasing (FXAA) to the swapchain. They are configured by the [`PostProcessSettings`](crate::post_process::PostProcessSettings) resource.
//!
//! To find out more about the deferred rendering pipeline and how to add custom render subpasses, see the documentation of the [`deferred`](crate::deferred) module.
use crate::prelude::*;
use bevy::prelude::*;

#[doc(hidden)]
pub mod prelude {
    pub use crate::deferred::*;
    pub use crate::passes::*;
    pub use crate::post_process::*;
    pub use crate::textures::*;
}

pub mod deferred;
mod passes;
pub mod post_process;
mod textures;

pub struct PbrPlugin;
impl Plugin for PbrPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(CorePlugin)
            .add_plugins(PassesPlugin)
            .add_plugins(PostProcessPlugin)
            .add_plugins(DeferredPlugin);
    }
}
