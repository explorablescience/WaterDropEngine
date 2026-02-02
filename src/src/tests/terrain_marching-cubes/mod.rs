use bevy::prelude::*;
use generate::MCGeneratePlugin;
use mc_chunk::MCChunkDescription;
use mc_compute_main::{MCComputeHandler, MCComputeHandlerGPU};
use process::MCProcessPlugin;
use render::MCRenderPlugin;
use spawn::MCSpawnPlugin;
use wde_render::core::{Extract, RenderApp};

mod mc_chunk;
mod mc_compute_main;
mod spawn;
mod generate;
mod process;
mod render;

/** Maximum LOD sudivision count */
pub const MC_MAX_SUB_COUNT: [u32; 3] = [20, 20, 20];
/** Maximum number of chunks to process per frame */
pub const MC_MAX_CHUNKS_PROCESS_PER_FRAME: usize = 2;
/** Maximum number of triangles allowed */
pub const MC_MAX_POINTS: u32 = MC_MAX_SUB_COUNT[0] * MC_MAX_SUB_COUNT[1] * MC_MAX_SUB_COUNT[2];
pub const MC_MAX_TRIANGLES: u32 = 20_000;

/**
 * This component is used to spawn the terrain spawner.
 * There should be only one terrain spawner in the scene.
 */
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(Transform)]
pub struct TerrainSpawner {
    /** The number of chunks to spawn in each direction (in a circle). */
    pub chunk_radius_count: i32,
    /** The real length of the chunk in each axis. */
    pub chunk_length: [f32; 3],
    /** The number of sub chunks to spawn in each chunk. */
    pub chunk_sub_count: [u32; 3],
    /** The iso level for the terrain generation. */
    pub iso_level: f32
}
impl Default for TerrainSpawner {
    fn default() -> Self {
        TerrainSpawner {
            chunk_radius_count: 6,
            chunk_length: [200.0, 200.0, 200.0],
            chunk_sub_count: MC_MAX_SUB_COUNT,
            iso_level: 0.0
        }
    }
}

pub struct TerrainPlugin;
impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        // Add the different generation plugins
        app
            .add_plugins(MCSpawnPlugin)
            .add_plugins(MCGeneratePlugin)
            .add_plugins(MCProcessPlugin)
            .add_plugins(MCRenderPlugin);

        // Add the compute main plugin
        app
            .init_resource::<MCComputeHandler>()
            .add_systems(Startup, MCComputeHandler::init);
        app.get_sub_app_mut(RenderApp).unwrap()
            .init_resource::<MCComputeHandlerGPU>()
            .add_systems(Extract, MCComputeHandler::extract);

        // Register the components to the reflect system
        app
            .register_type::<MCChunkDescription>()
            .register_type::<TerrainSpawner>();
    }
}
