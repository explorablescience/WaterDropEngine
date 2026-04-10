use bevy::{ecs::system::SystemParamItem, prelude::*};
use wde_renderer::prelude::*;
use wde_terrain::prelude::*;

use crate::{core::grid::GridChunkPos, prelude::Grid};

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UGridDescription {
    pub chunk_size: f32,
    pub subdivisions: f32,
    pub major_line_width: f32, // world units, e.g. 0.05
    pub minor_line_width: f32, // world units, e.g. 0.01
    pub major_color: [f32; 4],
    pub minor_color: [f32; 4],
    pub fade_center: [f32; 2], // world position of the center point for fading
    pub fade_start: f32,       // world distance from center point to start fading
    pub fade_end: f32          // world distance from center point to end fading
}
impl Default for UGridDescription {
    fn default() -> Self {
        Self {
            chunk_size: CHUNK_SIZE,
            subdivisions: CHUNK_RENDER_SUBDIVISIONS as f32,
            major_line_width: 0.03,
            minor_line_width: 0.01,
            major_color: [0.3, 0.3, 0.3, 0.6],
            minor_color: [0.2, 0.2, 0.2, 0.7],
            fade_center: [0.0, 0.0],
            fade_start: CHUNK_SIZE * 0.7 * (1.0 - 0.2),
            fade_end: CHUNK_SIZE * 0.7 * (1.0 + 0.2)
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UGridChunkPos {
    pub xz: [f32; 2]
}

#[derive(Asset, Clone, Debug, Default, TypePath)]
pub struct TerrainGridBuffer;
impl TerrainGridBuffer {
    pub const GRID_DESC_BIND: u32 = 0;
    pub const GRID_CHUNK_POS_BIND: u32 = 1;
}
impl RenderData for TerrainGridBuffer {
    type Params = ();

    fn describe(_params: &mut SystemParamItem<Self::Params>, builder: &mut RenderDataBuilder) {
        builder.add_buffer(
            Self::GRID_DESC_BIND,
            Buffer {
                label: "terrain-grid-description".to_string(),
                size: std::mem::size_of::<UGridDescription>(),
                usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST,
                content: Some(bytemuck::cast_slice(&[UGridDescription::default()]).into())
            }
        );

        // Create the list of chunk positions
        let mut chunks_pos = Vec::with_capacity((CHUNK_COUNT * CHUNK_COUNT) as usize);
        for y in -(CHUNK_COUNT as i32) / 2..CHUNK_COUNT as i32 / 2 {
            for x in -(CHUNK_COUNT as i32) / 2..CHUNK_COUNT as i32 / 2 {
                let pos = Grid::chunk_pos_to_world(GridChunkPos { x, y });
                chunks_pos.push(UGridChunkPos { xz: [pos.x, pos.y] });
            }
        }
        builder.add_buffer(
            Self::GRID_CHUNK_POS_BIND,
            Buffer {
                label: "terrain-grid-chunk-positions".to_string(),
                size: std::mem::size_of::<UGridChunkPos>() * (CHUNK_COUNT * CHUNK_COUNT) as usize,
                usage: BufferUsage::STORAGE,
                content: Some(bytemuck::cast_slice(&chunks_pos).into())
            }
        );
    }
}

#[derive(Asset, Clone, Debug, Default, TypePath)]
pub struct TerrainGridBufferBinding;
impl RenderBinding for TerrainGridBufferBinding {
    type Params = SRenderData<TerrainGridBuffer>;

    fn describe(
        &mut self,
        buffer: &SystemParamItem<Self::Params>,
        builder: &mut RenderBindingBuilder
    ) {
        builder.add_buffer(buffer, TerrainGridBuffer::GRID_DESC_BIND);
        builder.add_buffer(buffer, TerrainGridBuffer::GRID_CHUNK_POS_BIND);
    }

    fn label(&self) -> &str {
        "terrain_grid_buffer_binding"
    }
}

pub(crate) fn update_render(
    cursor_pos: Res<TerrainCursorPos>,
    terrain_buffer: ResRenderData<TerrainGridBuffer>,
    buffers: Res<RenderAssets<GpuBuffer>>,
    render_instance: Res<RenderInstance>
) {
    let terrain_buffer = match terrain_buffer.iter().next() {
        Some((_, buffer)) => buffer,
        None => return
    };
    if let Some(grid_desc_buffer) = buffers.get(
        &terrain_buffer
            .get_buffer(TerrainGridBuffer::GRID_DESC_BIND)
            .unwrap()
    ) {
        let grid_desc = UGridDescription {
            fade_center: [cursor_pos.world_pos.x, cursor_pos.world_pos.z],
            ..Default::default()
        };
        let render_instance = render_instance.0.read().unwrap();
        grid_desc_buffer
            .buffer
            .write(&render_instance, bytemuck::cast_slice(&[grid_desc]), 0);
    }
}
