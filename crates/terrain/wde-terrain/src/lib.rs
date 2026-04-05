//! This crate provides a terrain system for the game, including terrain management, rendering, and physics integration.
//! The terrain is divided into tiles, each of which has its own heightmap and splat maps for texture blending. The terrain system is designed to be efficient and flexible, allowing for dynamic updates to the terrain data and seamless integration with the rendering and physics systems.
//! The main components of the terrain system include:
//! - `Terrain`: The main terrain resource that holds all the terrain tiles and their data. It also manages the dirty tiles that need to be re-processed.
//! - `TerrainRenderer`: A component that holds the terrain tiles used for rendering, including their heightmaps and splat maps. It also manages the mapping from tile positions to their corresponding data and the list of dirty tiles that need to be re-processed for rendering.
//! - `TerrainPhysics`: A component that holds the terrain tiles used for physics, including their heightmaps. It also manages the mapping from tile positions to their corresponding data and the list of dirty tiles that need to be re-processed for physics.
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

pub mod prelude {
    pub use super::TerrainPlugin;
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
        // Add the terrain plugin and its dependencies
        app.add_plugins(TerrainRenderPlugin)
            .add_plugins(TerrainPhysicsPlugin)
            .add_plugins(TerrainUtilsPlugin);
    }
}
