use wde_renderer::prelude::*;
use bevy::{asset::io::embedded::GetAssetServer, prelude::*};

use crate::{manager::TILE_SIZE, render::passes::tiles_extractor::GpuTerrainTiles};

// The maximum number of terrain tiles that can be rendered
const MAX_TERRAIN_TILES: usize = 1000;

pub struct TerrainBufferPlugin;
impl Plugin for TerrainBufferPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TerrainBuffer>()
            .add_systems(Render, build_terrain_bind_group.in_set(RenderSet::BindGroups))
            .add_systems(Render, update_terrain_tiles_buffer.in_set(RenderSet::Prepare));
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TerrainDescription {
    pub tile_size: f32,
    pub _padding: f32,
}
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TerrainTileDescription {
    pub pos: [f32; 2],
    pub lod: f32,
    pub _padding: f32
}

/// Struct to hold the terrain uniform layout description.
#[derive(Resource)]
pub struct TerrainBuffer {
    pub desc_buffer: Handle<Buffer>,
    pub tiles_buffer: Handle<Buffer>,
    pub layout: BindGroupLayout,
    pub layout_built: WgpuBindGroupLayout,
    pub bind_group: Option<BindGroup>,
}
impl FromWorld for TerrainBuffer {
    fn from_world(world: &mut World) -> Self {
        let render_instance = world.get_resource::<RenderInstance>().unwrap();

        // Create the buffer
        let desc_buffer = world
            .get_asset_server()
            .add(Buffer {
                label: "ssbo-terrain-description-buffer".to_string(),
                size: std::mem::size_of::<TerrainDescription>(),
                usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST,
                content: Some(bytemuck::cast_slice(&[TerrainDescription {
                    tile_size: TILE_SIZE,
                    _padding: 0.0,
                }]).into()),
            });
        let tiles_buffer = world
            .get_asset_server()
            .add(Buffer {
                label: "ssbo-terrain-tiles-buffer".to_string(),
                size: std::mem::size_of::<TerrainTileDescription>() * MAX_TERRAIN_TILES,
                usage: BufferUsage::STORAGE | BufferUsage::COPY_DST,
                content: None,
            });

        // Create the terrain layout
        let layout = BindGroupLayout::new("terrain", |builder| {
            builder.add_buffer(
                0, ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                BufferBindingType::Uniform);
            builder.add_buffer(
                1, ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                BufferBindingType::Storage { read_only: true });
        });
        let layout_built = layout.build(&render_instance.0.read().unwrap());
        
        TerrainBuffer { desc_buffer, tiles_buffer, layout, layout_built, bind_group: None }
    }
}

fn build_terrain_bind_group(render_instance: Res<RenderInstance>, mut terrain_buffer: ResMut<TerrainBuffer>, buffers: Res<RenderAssets<GpuBuffer>>) {
    // Check if the bind group is already created
    if terrain_buffer.bind_group.is_some() {
        return;
    }

    // Create the bind group
    if let (Some(desc_buffer), Some(tiles_buffer)) = (
        buffers.get(&terrain_buffer.desc_buffer),
        buffers.get(&terrain_buffer.tiles_buffer)
    ) {
        let render_instance = render_instance.0.read().unwrap();
        let bind_group = BindGroupBuilder::build("terrain", &render_instance, &terrain_buffer.layout_built, &vec![
            BindGroupBuilder::buffer(0, &desc_buffer.buffer),
            BindGroupBuilder::buffer(1, &tiles_buffer.buffer)
        ]);
        terrain_buffer.bind_group = Some(bind_group);
    }
}

// System to update the terrain tiles buffer with the current visible tiles
fn update_terrain_tiles_buffer(
    render_instance: Res<RenderInstance>, terrain_buffer: Res<TerrainBuffer>, buffers: Res<RenderAssets<GpuBuffer>>,
    terrain_tiles: Res<GpuTerrainTiles>
) {
    // Check if the bind group is already created
    if terrain_buffer.bind_group.is_none() {
        return;
    }

    // Get the buffer
    let tile_buffer = match buffers.get(&terrain_buffer.tiles_buffer) {
        Some(buffer) => buffer,
        None => return,
    };

    // Prepare the data
    let data: Vec<TerrainTileDescription> = terrain_tiles.ready_tiles.iter().map(|tile| {
        TerrainTileDescription {
            pos: [tile.position.x as f32, tile.position.y as f32],
            lod: 1.0,
            _padding: 0.0
        }
    }).collect();

    // Update the buffer
    let render_instance = render_instance.0.read().unwrap();
    tile_buffer.buffer.write(&render_instance, bytemuck::cast_slice(&data), 0);
}
