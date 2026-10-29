//! Terrain system for WaterDropEngine.
//!
//! The world is a square of `CHUNK_COUNT` × `CHUNK_COUNT` chunks of `CHUNK_SIZE` world units.
//! Each chunk has a heightmap and splat maps blending the ground materials, stored as layers of
//! shared GPU texture arrays.
//!
//! A terrain lives in a folder of `res/`, described by its `config/terrain.json`
//! ([`TerrainConfig`](prelude::TerrainConfig)): height encoding, map files, and the rules of its
//! masks. It is a single entity made of:
//! - [`Terrain`](prelude::Terrain): the source of truth. Loaded from the heightmaps and splat maps
//!   of its folder, it answers height queries ([`Terrain::height_at`](prelude::Terrain::height_at))
//!   and queues the chunks edited since the last frame for the render and physics. From the
//!   heights, it derives a slope and a buildable mask: buildable ground is flat ground at height 0
//!   away from any other ([`Terrain::is_area_buildable`](prelude::Terrain::is_area_buildable)),
//!   walkable ground is not steeper than the config allows
//!   ([`Terrain::is_walkable`](prelude::Terrain::is_walkable)).
//! - [`TerrainRenderer`](prelude::TerrainRenderer): draws the visible chunks with levels of detail
//!   depending on their distance to the camera, optionally tinted by the masks
//!   ([`TerrainDebugView`](prelude::TerrainDebugView)).
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
//!     let terrain = Terrain::load(DEFAULT_TERRAIN_PATH);
//!     commands.spawn((
//!         TerrainRenderer::new(&asset_server, &terrain),
//!         terrain,
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

pub(crate) mod config;
pub(crate) mod manager;
pub(crate) mod masks;
pub(crate) mod physics;
pub mod render;
pub(crate) mod utils;

#[doc(hidden)]
pub mod prelude {
    pub use crate::config::{TERRAIN_CONFIG_FILE, TerrainConfig, TerrainMaskRules};
    pub use crate::manager::*;
    pub use crate::masks::ChunkMasks;
    pub use crate::physics::terrain_physics::TerrainPhysics;
    pub use crate::render::dependencies::terrain_buffer::{
        TERRAIN_MATERIAL_COUNT, TerrainDebugView, TerrainRenderSettings
    };
    pub use crate::render::extractor::*;
    pub use crate::render::renderer::TERRAIN_LOD_COUNT;
    pub use crate::render::renderer::TerrainRenderer;
    pub use crate::render::renderer_gpu::{
        TerrainChunkArrayBg, TerrainComputeArrayBg, TerrainRendererGPU
    };
    pub use crate::utils::cursor_pos::TerrainCursorPos;
    pub use crate::{BUILDINGS_COLLIDER_GROUP, HEIGHTMAP_COLLIDER_GROUP, MODULES_COLLIDER_GROUP};
}

pub struct TerrainPlugin;
impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            TerrainRenderPlugin,
            TerrainPhysicsPlugin,
            TerrainUtilsPlugin
        ))
        .register_type::<prelude::Terrain>()
        .register_type::<prelude::TerrainRenderer>()
        .register_type::<prelude::TerrainPhysics>()
        .register_type::<prelude::TerrainCursorPos>();
    }
}
