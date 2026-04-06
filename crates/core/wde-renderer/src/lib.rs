//! The main renderer plugin and related utilities.
//!
//! This crate provides the main rendering plugin for the engine, as well as core types and utilities for defining render pipelines, render passes and related resources. It is designed to be flexible and extensible, allowing users to define their own render pipelines and passes while providing a solid foundation of core functionality.
//! It is an overlay over [`wde_wgpu`] that integrates it with the engine's asset system, ECS and render graph.
//!
//! The main components of this crate are:
//! - The [`RenderPlugin`] that sets up the rendering system and adds the necessary plugins and resources. It is the main entry point for using the renderer and should be added to the app to enable rendering.
//! - The [`assets`] module that defines the render pipeline asset system and related types.
//! - The [`core`] module that defines core types and resources for the renderer, such as the render graph and pipeline manager.
//! - The [`passes`] module that defines the traits and types for render passes and sub-passes, as well as a simple render graph implementation.
//! - The [`utils`] module that provides utility resources and systems, such as the depth texture and meshes for rendering.

#[doc(hidden)]
pub mod prelude {
    pub use crate::assets::*;
    pub use crate::core::*;
    pub use crate::passes::*;
    pub use crate::utils::*;

    // TODO: Remove this
    pub use wde_wgpu::bind_group::{
        BindGroupBuilder, BindGroupLayout, BindGroupLayoutBuilder, WgpuBindGroup as BindGroup,
        WgpuBindGroupLayout
    };
}

pub mod wgpu_utils {
    pub use wde_wgpu::command_buffer::{CommandBuffer, RenderPassBuilder, RenderPassColorAttachment, RenderPassDepth};
}

pub mod assets;
pub mod core;
pub mod passes;
pub mod utils;

use crate::{assets::AssetsPlugin, core::RenderCorePlugin, utils::UtilsPlugin};
use bevy::prelude::*;

pub struct RenderPlugin;
impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        // First, add the core plugin
        app.add_plugins(RenderCorePlugin);

        // Then add the other plugins
        app.add_plugins(UtilsPlugin).add_plugins(AssetsPlugin);
    }
}
