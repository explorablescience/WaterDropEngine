//! Utility functions and resources for the renderer.
//!
//! This includes:
//! - The mesh used for rendering with SSBOs, described in [`SsboMesh`].
//! - The mesh used for post-processing passes, described in [`PostProcessingMesh`].
//! - The transform uniform utility resource, described in [`TransformUniform`].
//! - The lightweight color helper enum, described in [`Color`].

use bevy::prelude::*;

use crate::{
    assets::RenderBindingPluginRegisterOld,
    core::{ExtractResourcePlugin, RenderApp},
    utils::ssbo_mesh::SsboMeshDescriptor
};

mod color;
mod post_process_mesh;
pub(crate) mod ssbo_mesh;

pub use color::Color;
pub use post_process_mesh::PostProcessingMesh;
pub use ssbo_mesh::SsboMesh;

/** Multisample anti-aliasing sample count used throughout the renderer. */
pub const MSAA_SAMPLE_COUNT: u32 = 4;

pub(crate) struct UtilsPlugin;
impl Plugin for UtilsPlugin {
    fn build(&self, app: &mut App) {
        // Add the ssbo
        app.add_plugins(RenderBindingPluginRegisterOld::<SsboMesh>::default());
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .init_resource::<SsboMeshDescriptor>();

        // Add the creation of the mesh for post-processing passes
        app.init_resource::<PostProcessingMesh>()
            .add_systems(Startup, PostProcessingMesh::init)
            .add_plugins(ExtractResourcePlugin::<PostProcessingMesh>::default());
    }
}
