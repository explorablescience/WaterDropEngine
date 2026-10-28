//! Terrain system for WaterDropEngine.
//!
//! The world is a square of `CHUNK_COUNT` × `CHUNK_COUNT` chunks of `CHUNK_SIZE` world units.
//! Each chunk has a heightmap and splat maps blending the ground materials, stored as layers of
//! shared GPU texture arrays.
//!
//! A terrain is a single entity made of:
//! - [`Terrain`](prelude::Terrain): the source of truth. Loaded from
//!   `res/<path>/heightmap_x_z.png` and `splatmap_x_z-<k>.png`, it answers height queries
//!   ([`Terrain::height_at`](prelude::Terrain::height_at)) and queues the chunks edited since the
//!   last frame for the render and physics.
//! - [`TerrainRenderer`](prelude::TerrainRenderer): draws the visible chunks with levels of detail
//!   depending on their distance to the camera.
//! - [`TerrainPhysics`](prelude::TerrainPhysics): one heightfield collider per chunk, in
//!   [`HEIGHTMAP_COLLIDER_GROUP`].
//!
//! The [`TerrainCursorPos`](prelude::TerrainCursorPos) resource gives the terrain point under the
//! cursor, in both the main and render worlds. The [`BUILDINGS_COLLIDER_GROUP`] and
//! [`MODULES_COLLIDER_GROUP`] are reserved for the objects placed on the terrain (see
//! `wde-terrain-grid`).
//!
//! # Example
//! ```rust,ignore
//! fn init(mut commands: Commands, asset_server: Res<AssetServer>) {
//!     commands.spawn((
//!         Terrain::load(DEFAULT_TERRAIN_PATH),
//!         TerrainRenderer::new(&asset_server),
//!         TerrainPhysics::default()
//!     ));
//! }
//! ```
//! `wde-terrain-editor` spawns this default terrain, and edits it.

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
