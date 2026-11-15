//! Physically based rendering (PBR) integration for WaterDropEngine.
//!
//! This crate wires the PBR rendering stack into the engine and exposes the core
//! building blocks to configure deferred PBR rendering.
//!
//! The PBR rendering stack is built on top of the deferred rendering pipeline, and consists of the following passes:
//! - [`RenderPassDeferredGBuffer`](crate::passes::RenderPassDeferredGBuffer): Renders the scene geometry into the G-buffer (only opaque objects). It writes to the [`DepthTexture`](crate::textures::DepthTexture) and the G-buffer [`DeferredTextures`](crate::textures::DeferredTextures) (albedo, normal, depth) in MSAA format.
//! - [`RenderPassDeferredLighting`](crate::passes::RenderPassDeferredLighting): Performs the lighting pass, reading from the resolved G-buffer textures [`DeferredTexturesResolved`](crate::textures::DeferredTexturesResolved) and writing to a [`RenderTexture`](crate::textures::RenderTexture) in MSAA format.
//! - [`RenderPassTransparent`](crate::passes::RenderPassTransparent): Renders transparent objects, using the [`DepthTexture`](crate::textures::DepthTexture) for depth testing, and writing to the same [`RenderTexture`](crate::textures::RenderTexture) as the lighting pass (in MSAA format).
//! - The [`post_process`](crate::post_process) passes: fog (before the transparent objects), then the resolve of the HDR [`RenderTexture`](crate::textures::RenderTexture), the bloom, and the final composite (color grading, tonemapping, ...) to the swapchain. They are configured by the [`PostProcessSettings`](crate::post_process::PostProcessSettings) resource.
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
