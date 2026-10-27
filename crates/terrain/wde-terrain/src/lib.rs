//! Terrain system for WaterDropEngine.
//!
//! Divide the world into a grid of chunks, each with its own heightmap and splat-map stored
//! as a layer inside shared GPU texture arrays.
//!
//! # Example
//! ```
//! fn init(mut commands: Commands, asset_server: Res<AssetServer>) {
//!     commands.spawn((
//!         Terrain::load(DEFAULT_TERRAIN_PATH),
//!         TerrainRenderer::new(&asset_server),
//!         TerrainPhysics::default()
//!     ));
//! }
//! ```

use bevy::prelude::*;
use wde_physics::prelude::*;

use crate::{
    physics::TerrainPhysicsPlugin, render::TerrainRenderPlugin, utils::TerrainUtilsPlugin
};

/// The terrain heightmap.
pub const HEIGHTMAP_COLLIDER_GROUP: ColliderGroup = ColliderGroup::GROUP_1;
/// The buildings placed on the terrain.
pub const BUILDINGS_COLLIDER_GROUP: ColliderGroup = ColliderGroup::GROUP_2;
/// Modules placed inside buildings.
pub const MODULES_COLLIDER_GROUP: ColliderGroup = ColliderGroup::GROUP_3;

pub(crate) mod manager;
pub(crate) mod physics;
pub mod render;
pub(crate) mod utils;

#[doc(hidden)]
pub mod prelude {
    pub use crate::manager::*;
    pub use crate::physics::terrain_physics::TerrainPhysics;
    pub use crate::render::dependencies::terrain_buffer::{
        TERRAIN_MATERIAL_COUNT, TerrainRenderSettings
    };
    pub use crate::render::extractor::*;
    pub use crate::render::renderer::TERRAIN_LOD_COUNT;
    pub use crate::render::renderer::TerrainRenderer;
    pub use crate::render::renderer_gpu::{
        TerrainChunkArrayBg, TerrainComputeArrayBg, TerrainRendererGPU
    };
    pub use crate::utils::cursor_pos::TerrainCursorPos;
    pub use crate::{
        BUILDINGS_COLLIDER_GROUP, HEIGHTMAP_COLLIDER_GROUP, MODULES_COLLIDER_GROUP
    };
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
