use wde_renderer::prelude::*;
use wde_terrain::prelude::*;
use bevy::{asset::io::embedded::GetAssetServer, prelude::*};

use crate::{core::grid::GridChunkPos, prelude::Grid};

pub struct TerrainGridBufferPlugin;
impl Plugin for TerrainGridBufferPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<TerrainGridBuffer>()
            .add_systems(Render, build.in_set(RenderSet::BindGroups));
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UGridDescription {
    pub chunk_size:        f32,
    pub subdivisions:      f32,
    pub major_line_width:  f32,  // world units, e.g. 0.05
    pub minor_line_width:  f32,  // world units, e.g. 0.01
    pub major_color:       [f32; 4],
    pub minor_color:       [f32; 4],
}
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UGridChunkPos {
    pub xz: [f32; 2],
}

#[derive(Resource)]
pub struct TerrainGridBuffer {
    pub grid_desc_buffer: Handle<Buffer>,
    pub grid_chunk_pos_buffer: Handle<Buffer>,
    pub layout: BindGroupLayout,
    pub layout_built: WgpuBindGroupLayout,
    pub bind_group: Option<BindGroup>
}
impl FromWorld for TerrainGridBuffer {
    fn from_world(world: &mut World) -> Self {
        // Create the list of chunk positions
        let mut chunks_pos = Vec::with_capacity((CHUNK_COUNT * CHUNK_COUNT) as usize);
        for y in -(CHUNK_COUNT as i32) / 2 .. CHUNK_COUNT as i32 / 2 {
            for x in -(CHUNK_COUNT as i32) / 2 .. CHUNK_COUNT as i32 / 2 {
                let pos = Grid::chunk_pos_to_world(GridChunkPos { x, y });
                chunks_pos.push(UGridChunkPos {
                    xz: [pos.x, pos.y]
                });
            }
        }

        // Create the buffers
        let grid_desc = world
            .get_asset_server()
            .add(Buffer {
                label: "terrain-grid-description".to_string(),
                size: std::mem::size_of::<UGridDescription>(),
                usage: BufferUsage::UNIFORM,
                content: Some(bytemuck::cast_slice(&[UGridDescription {
                    chunk_size: CHUNK_SIZE,
                    subdivisions: CHUNK_RENDER_SUBDIVISIONS as f32,
                    major_line_width: 0.05,
                    minor_line_width: 0.01,
                    major_color: [1.0, 1.0, 1.0, 1.0],
                    minor_color: [0.5, 0.5, 0.5, 1.0],
                }]).into()),
            });
        let grid_chunk_pos = world
            .get_asset_server()
            .add(Buffer {
                label: "terrain-grid-chunk-positions".to_string(),
                size: std::mem::size_of::<UGridChunkPos>() * (CHUNK_COUNT * CHUNK_COUNT) as usize,
                usage: BufferUsage::STORAGE,
                content: Some(bytemuck::cast_slice(&chunks_pos).into()),
            });

        // Create the layouts
        let layout = BindGroupLayout::new("terrain-grid", |builder| {
            builder.add_buffer(
                0, ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                BufferBindingType::Uniform);
            builder.add_buffer(
                1, ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                BufferBindingType::Storage { read_only: true });
        });
        let render_instance = world.get_resource::<RenderInstance>().unwrap();
        let layout_built = layout.build(&render_instance.0.read().unwrap());
        
        Self { grid_desc_buffer: grid_desc, grid_chunk_pos_buffer: grid_chunk_pos, layout, layout_built, bind_group: None }
    }
}

fn build(render_instance: Res<RenderInstance>, mut terrain_buffer: ResMut<TerrainGridBuffer>, buffers: Res<RenderAssets<GpuBuffer>>) {
    // Check if the bind group is already created
    if terrain_buffer.bind_group.is_some() {
        return;
    }

    // Create the bind group
    if let (Some(grid_desc_buffer), Some(grid_chunk_pos_buffer)) = (
        buffers.get(&terrain_buffer.grid_desc_buffer),
        buffers.get(&terrain_buffer.grid_chunk_pos_buffer)
    ) {
        let render_instance = render_instance.0.read().unwrap();
        let bind_group = BindGroupBuilder::build("terrain-grid", &render_instance, &terrain_buffer.layout_built, &vec![
            BindGroupBuilder::buffer(0, &grid_desc_buffer.buffer),
            BindGroupBuilder::buffer(1, &grid_chunk_pos_buffer.buffer)
        ]);
        terrain_buffer.bind_group = Some(bind_group);
    }
}
