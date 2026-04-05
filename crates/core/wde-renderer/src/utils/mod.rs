//! Utility functions and resources for the renderer.
//!
//! This includes:
//! - The depth texture [`DepthTexture`] that is used for depth testing in the main render pass.
//! - The mesh used for rendering with SSBOs, described in [`SsboMesh`].
//! - The mesh used for post-processing passes, described in [`PostProcessingMesh`].
//! - The transform uniform utility resource, described in [`TransformUniform`].

use bevy::prelude::*;

use crate::{
    assets::RenderBindingPluginRegister,
    core::{ExtractResourcePlugin, RenderApp}
};

mod depth;
mod post_process_mesh;
mod ssbo_mesh;
mod transform;

pub use depth::*;
pub use post_process_mesh::*;
pub use ssbo_mesh::*;
pub use transform::*;

/** Multisample anti-aliasing sample count used throughout the renderer. */
pub const MSAA_SAMPLE_COUNT: u32 = 4;

pub(crate) struct UtilsPlugin;
impl Plugin for UtilsPlugin {
    fn build(&self, app: &mut App) {
        // Add the depth texture to the app
        app.add_plugins(DepthTexturePlugin);

        // Add the ssbo
        app.add_plugins(RenderBindingPluginRegister::<SsboMesh>::default());
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .init_resource::<SsboMeshDescriptor>();

        // Add the creation of the mesh for post-processing passes
        app.init_resource::<PostProcessingMesh>()
            .add_systems(Startup, PostProcessingMesh::init)
            .add_plugins(ExtractResourcePlugin::<PostProcessingMesh>::default());
    }
}
