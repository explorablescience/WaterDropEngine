//! This crate provides a terrain system for the game.
//! The terrain is divided into tiles, each of which has its own heightmap and splat maps for texture blending. The terrain system is designed to be efficient and flexible, allowing for dynamic updates to the terrain data and seamless integration with the rendering and physics systems.
//!
//! This module is still WIP.
//!
//! # Example
//! ```
//! fn init(mut commands: Commands, asset_server: Res<AssetServer>) {
//!     // Load the terrain from the specified path
//!     commands.spawn((
//!         Terrain::load("tests/terrain"),
//!         TerrainRenderer::new(&asset_server),
//!         TerrainPhysics::default()
//!     ));
//! }

use bevy::prelude::*;

use crate::{
    physics::TerrainPhysicsPlugin, render::TerrainRenderPlugin, utils::TerrainUtilsPlugin
};

pub(crate) mod manager;
pub(crate) mod physics;
pub(crate) mod render;
pub(crate) mod utils;

#[doc(hidden)]
pub mod prelude {
    pub use crate::manager::*;
    pub use crate::physics::terrain_physics::TerrainPhysics;
    pub use crate::render::extractor::*;
    pub use crate::render::renderer::TerrainRenderer;
    pub use crate::render::renderer_gpu::TerrainRendererGPU;
    pub use crate::utils::cursor_pos::TerrainCursorPos;
}

pub struct TerrainPlugin;
impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            TerrainRenderPlugin,
            TerrainPhysicsPlugin,
            TerrainUtilsPlugin
        ));
    }
}
