use bevy::{ecs::system::SystemParamItem, prelude::*};
use wde_renderer::prelude::*;

use crate::{
    manager::{CHUNK_HEIGHT, CHUNK_RENDER_SUBDIVISIONS, CHUNK_SIZE},
    render::renderer_gpu::TerrainRendererGPU
};

// The maximum number of terrain tiles that can be rendered
const MAX_TERRAIN_TILES: usize = 1000;

/// Runtime-editable terrain rendering parameters, synced to the render world each frame.
#[derive(Resource, ExtractResource, Clone, Debug)]
pub struct TerrainRenderSettings {
    pub displacement_scales: [f32; 4],
    pub tiling_scales: [f32; 4]
}
impl Default for TerrainRenderSettings {
    fn default() -> Self {
        Self {
            displacement_scales: [0.0, 0.16, 0.07, 0.0],
            tiling_scales: [1.0, 9.5, 8.5, 1.0]
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TerrainDescription {
    pub tile_size: [f32; 3],
    pub tile_subdivisions: f32,
    /// Displacement scale (metres) applied per splat layer (indices 0–3).
    pub displacement_scales: [f32; 4],
    /// UV tiling repetitions across one tile per splat layer (indices 0–3).
    pub tiling_scales: [f32; 4]
}
impl Default for TerrainDescription {
    fn default() -> Self {
        Self {
            tile_size: [CHUNK_SIZE, CHUNK_HEIGHT, CHUNK_SIZE],
            tile_subdivisions: CHUNK_RENDER_SUBDIVISIONS as f32,
            displacement_scales: TerrainRenderSettings::default().displacement_scales,
            tiling_scales: TerrainRenderSettings::default().tiling_scales
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TerrainTileDescription {
    pub pos: [f32; 2],
    pub lod: f32,
    pub _padding: f32
}

/// Struct to hold the terrain uniform layout description.
#[derive(Asset, Clone, Debug, Default, TypePath)]
pub struct TerrainBuffer;
impl TerrainBuffer {
    pub const DESC_BIND: u32 = 0;
    pub const TILES_BIND: u32 = 1;
}
impl RenderData for TerrainBuffer {
    type Params = ();

    fn describe(_params: &mut SystemParamItem<Self::Params>, builder: &mut RenderDataBuilder) {
        builder.add_buffer(
            Self::DESC_BIND,
            Buffer {
                label: "ssbo-terrain-description-buffer".to_string(),
                size: std::mem::size_of::<TerrainDescription>(),
                usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST,
                content: Some(
                    bytemuck::cast_slice(&[TerrainDescription::default()])
                    .into()
                )
            }
        );
        builder.add_buffer(
            Self::TILES_BIND,
            Buffer {
                label: "ssbo-terrain-tiles-buffer".to_string(),
                size: std::mem::size_of::<TerrainTileDescription>() * MAX_TERRAIN_TILES,
                usage: BufferUsage::STORAGE | BufferUsage::COPY_DST,
                content: None
            }
        );
    }
}

#[derive(Asset, Clone, TypePath, Default)]
pub struct TerrainBufferBinding;
impl RenderBinding for TerrainBufferBinding {
    type Params = SRenderData<TerrainBuffer>;

    fn describe(
        &mut self,
        buffer: &SystemParamItem<Self::Params>,
        builder: &mut RenderBindingBuilder
    ) {
        builder.add_buffer(buffer, TerrainBuffer::DESC_BIND);
        builder.add_buffer(buffer, TerrainBuffer::TILES_BIND);
    }

    fn label(&self) -> &str {
        "terrain_buffer_binding"
    }
}

// System to write the current TerrainRenderSettings into the description uniform buffer
pub(crate) fn update_terrain_description_buffer(
    render_instance: Res<RenderInstance>,
    terrain_buffer: ResRenderData<TerrainBuffer>,
    buffers: Res<RenderAssets<GpuBuffer>>,
    settings: Res<TerrainRenderSettings>
) {
    if !settings.is_changed() {
        return;
    }
    let terrain_buffer = match terrain_buffer.iter().next() {
        Some((_, buf)) => buf,
        None => return
    };
    let desc_buffer = match buffers.get(
        &terrain_buffer.get_buffer(TerrainBuffer::DESC_BIND).unwrap()
    ) {
        Some(buf) => buf,
        None => return
    };
    let render_instance = render_instance.0.read().unwrap();
    desc_buffer.buffer.write(
        &render_instance,
        bytemuck::cast_slice(&[TerrainDescription {
            tile_size: [CHUNK_SIZE, CHUNK_HEIGHT, CHUNK_SIZE],
            tile_subdivisions: CHUNK_RENDER_SUBDIVISIONS as f32,
            displacement_scales: settings.displacement_scales,
            tiling_scales: settings.tiling_scales
        }]),
        0
    );
}

// System to update the terrain tiles buffer with the current visible tiles
pub(crate) fn update_terrain_tiles_buffer(
    render_instance: Res<RenderInstance>,
    terrain_buffer: ResRenderData<TerrainBuffer>,
    buffers: Res<RenderAssets<GpuBuffer>>,
    terrain_tiles: Res<TerrainRendererGPU>,
    mut is_set: Local<bool>
) {
    // Check if is ready
    if *is_set || !terrain_tiles.ready {
        return;
    }

    // Get the buffer
    let terrain_buffer = match terrain_buffer.iter().next() {
        Some((_, buffer)) => buffer,
        None => return
    };
    let tile_buffer = match buffers.get(
        &terrain_buffer
            .get_buffer(TerrainBuffer::TILES_BIND)
            .unwrap()
    ) {
        Some(buffer) => buffer,
        None => return
    };

    // Prepare the data
    let data: Vec<TerrainTileDescription> = terrain_tiles
        .tiles
        .iter()
        .map(|tile| TerrainTileDescription {
            pos: [tile.position.x as f32, tile.position.y as f32],
            lod: 1.0,
            _padding: 0.0
        })
        .collect();

    // Update the buffer
    let render_instance = render_instance.0.read().unwrap();
    tile_buffer
        .buffer
        .write(&render_instance, bytemuck::cast_slice(&data), 0);

    *is_set = true;
}
