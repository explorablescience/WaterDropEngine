//! The main renderer plugin and related utilities.
//!
//! This crate provides the main rendering plugin for the engine, as well as core types and utilities for defining render pipelines, render passes and related resources. It is designed to be flexible and extensible, allowing users to define their own render pipelines and passes while providing a solid foundation of core functionality.
//! It is an overlay over [`wde_wgpu`](wde_wgpu) that integrates it with the engine's asset system, ECS and render graph.
//!
//! The renderer flow is composed of the following stages:
//! ```text
//! |--------------------------------------------------------------------|
//! |      |         |          Main World Update                        |
//! | Sync | Extract |---------------------------------------------------|
//! |      |         |         Render World Update                       |
//! |--------------------------------------------------------------------|
//! ```
//! Where:
//! - The **Sync** stage is responsible for synchronizing entities between the main and render worlds (see the [`sync`](crate::sync) module for more details).
//! - The **Extract** stage is responsible for extracting resources, queries and entities from the main world to the render world, so that they can be used by the render passes (see the [`core`](crate::core) and [`sync`](crate::sync) modules for more details).
//! - The **Main World Update** and **Render World Update** stages are responsible for running the systems in the main and render worlds, respectively.
//!
//! The main components of this crate are:
//! - The [`assets`](crate::assets) module that defines the render pipeline asset system and related types.
//! - The [`core`](crate::core) module that defines core types and resources for the renderer, such as the render graph and pipeline manager.
//! - The [`passes`](crate::passes) module that defines the traits and types for render passes and sub-passes, as well as a simple render graph implementation.
//! - The [`sync`](crate::sync) module that provides utilities to automatically extract resources, query and entities from the main to the render world.
//! - The [`utils`](crate::utils) module that provides utility resources and systems, such as the depth texture and meshes for rendering.

extern crate self as wde_renderer;

#[doc(hidden)]
pub mod prelude {
    pub use crate::assets::*;
    pub use crate::core::*;
    pub use crate::passes::*;
    pub use crate::sync::*;
    pub use crate::utils::*;

    // Re-export the macros
    pub use wde_renderer_macros::*;
}

pub mod wgpu_utils {
    pub use wde_wgpu::buffer::AsyncReadback;
    pub use wde_wgpu::command_buffer::{
        CommandBuffer, RenderPassBuilder, RenderPassColorAttachment, RenderPassDepth
    };
}

pub mod assets;
pub mod core;
pub mod passes;
pub mod sync;
pub mod utils;

use crate::{assets::AssetsPlugin, core::RenderCorePlugin, sync::SyncPlugin, utils::UtilsPlugin};
use bevy::prelude::*;

pub struct RenderPlugin;
impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        // First, add the core plugin
        app.add_plugins(RenderCorePlugin);

        // Finally, add the other plugins
        app.add_plugins((SyncPlugin, UtilsPlugin, AssetsPlugin));
    }
}
