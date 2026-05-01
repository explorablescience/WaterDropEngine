use bevy::{ecs::system::SystemParamItem, prelude::*};
use wde_renderer::prelude::*;

use crate::{
    manager::{CHUNK_HEIGHT, CHUNK_RENDER_SUBDIVISIONS, CHUNK_SIZE},
    render::renderer_gpu::TerrainRendererGPU
};

const MAX_TERRAIN_TILES: usize = 1000;

/// Runtime-editable terrain rendering parameters synced to the render world each frame.
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
    pub displacement_scales: [f32; 4],
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

/// Per-tile GPU descriptor.
/// `tile_layer` maps this draw-call instance to its layer in the texture arrays.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TerrainTileDescription {
    pub pos: [f32; 2],
    pub lod: f32,
    pub tile_layer: u32
}

#[derive(Asset, Clone, Debug, Default, TypePath)]
pub struct TerrainBuffer;
impl TerrainBuffer {
    pub const DESC_BIND: u32 = 0;
    pub const TILES_BIND: u32 = 1;
}
impl RenderData for TerrainBuffer {
    type Params = ();

    fn describe(_params: &mut SystemParamItem<Self::Params>, builder: &mut RenderDataBuilder) {
        builder
            .add_buffer(
                Self::DESC_BIND,
                Buffer {
                    label: "terrain-description-buffer".to_string(),
                    size: std::mem::size_of::<TerrainDescription>(),
                    usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST,
                    content: Some(bytemuck::cast_slice(&[TerrainDescription::default()]).into())
                }
            )
            .add_buffer(
                Self::TILES_BIND,
                Buffer {
                    label: "terrain-tiles-buffer".to_string(),
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
        builder
            .add_buffer(buffer, TerrainBuffer::DESC_BIND)
            .add_buffer(buffer, TerrainBuffer::TILES_BIND);
    }

    fn label(&self) -> &str {
        "terrain_buffer_binding"
    }
}

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
        Some((_, b)) => b,
        None => return
    };
    let buf = match buffers.get(&terrain_buffer.get_buffer(TerrainBuffer::DESC_BIND).unwrap()) {
        Some(b) => b,
        None => return
    };
    let render_instance = render_instance.0.read().unwrap();
    buf.buffer.write(
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

pub(crate) fn update_terrain_tiles_buffer(
    render_instance: Res<RenderInstance>,
    terrain_buffer: ResRenderData<TerrainBuffer>,
    buffers: Res<RenderAssets<GpuBuffer>>,
    terrain: Res<TerrainRendererGPU>,
    mut is_set: Local<bool>
) {
    if *is_set || !terrain.ready {
        return;
    }

    let terrain_buffer = match terrain_buffer.iter().next() {
        Some((_, b)) => b,
        None => return
    };
    let tile_buffer = match buffers.get(
        &terrain_buffer
            .get_buffer(TerrainBuffer::TILES_BIND)
            .unwrap()
    ) {
        Some(b) => b,
        None => return
    };

    // Build sorted slice: layer index is the draw instance index.
    let mut tiles: Vec<(&crate::manager::ChunkPos, &u32)> = terrain.pos_to_layer.iter().collect();
    tiles.sort_by_key(|&(_, layer)| *layer);

    let data: Vec<TerrainTileDescription> = tiles
        .iter()
        .map(|&(pos, layer)| TerrainTileDescription {
            pos: [pos.x as f32, pos.y as f32],
            lod: 1.0,
            tile_layer: *layer
        })
        .collect();

    let render_instance = render_instance.0.read().unwrap();
    tile_buffer
        .buffer
        .write(&render_instance, bytemuck::cast_slice(&data), 0);

    *is_set = true;
}
