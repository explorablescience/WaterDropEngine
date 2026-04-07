//! Physically based rendering (PBR) integration for WaterDropEngine.
//!
//! This crate wires the PBR rendering stack into the engine and exposes the core
//! building blocks to configure deferred PBR rendering.
//!
//! The PBR rendering stack is built on top of the deferred rendering pipeline, and consists of the following passes:
//! - [`RenderPassDeferredGBuffer`](crate::passes::RenderPassDeferredGBuffer): Renders the scene geometry into the G-buffer (only opaque objects). It writes to the [`DepthTexture`](crate::textures::DepthTexture) and the G-buffer [`DeferredTextures`](crate::textures::DeferredTextures) (albedo, normal, depth) in MSAA format.
//! - [`RenderPassDeferredLighting`](crate::passes::RenderPassDeferredLighting): Performs the lighting pass, reading from the resolved G-buffer textures [`DeferredTexturesResolved`](crate::textures::DeferredTexturesResolved) and writing to a [`RenderTexture`](crate::textures::RenderTexture) in MSAA format.
//! - [`RenderPassTransparent`](crate::passes::RenderPassTransparent): Renders transparent objects, using the [`DepthTexture`](crate::textures::DepthTexture) for depth testing, and writing to the same [`RenderTexture`](crate::textures::RenderTexture) as the lighting pass (in MSAA format).
//! - `RenderPassResolve`: At the end of the frame, resolves the MSAA [`RenderTexture`](crate::textures::RenderTexture) to the swapchain image for presentation. This pass is automatically added by the engine and cannot be configured.
//!
//! To find out more about the deferred rendering pipeline and how to add custom render subpasses, see the documentation of the [`deferred`](crate::deferred) module.
use crate::prelude::*;
use bevy::prelude::*;

#[doc(hidden)]
pub mod prelude {
    pub use crate::deferred::*;
    pub use crate::passes::*;
    pub use crate::textures::*;
}

pub mod deferred;
mod passes;
mod textures;

pub struct PbrPlugin;
impl Plugin for PbrPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(CorePlugin)
            .add_plugins(PassesPlugin)
            .add_plugins(DeferredPlugin);
    }
}
