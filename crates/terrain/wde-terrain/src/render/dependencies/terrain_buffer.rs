use bevy::{ecs::system::SystemParamItem, prelude::*};
use wde_renderer::prelude::*;

use crate::{
    config::TerrainConfig,
    manager::{
        CHUNK_MAP_SIZE, CHUNK_RENDER_SUBDIVISIONS, CHUNK_SIZE, SPLAT_MAP_COUNT, SPLAT_MAPS, Terrain
    },
    render::renderer_gpu::TerrainRendererGPU
};

const MAX_TERRAIN_TILES: usize = 1024;
/// Number of terrain materials.
pub const TERRAIN_MATERIAL_COUNT: usize = (SPLAT_MAPS * SPLAT_MAP_COUNT) as usize;

/// Runtime-editable terrain rendering parameters synced to the render world each frame. The
/// material parameters are reset from the [`TerrainConfig`] of each spawned terrain.
#[derive(Resource, ExtractResource, Clone, Debug)]
pub struct TerrainRenderSettings {
    /// World size covered by one repeat of each material's textures.
    pub tiling: [f32; TERRAIN_MATERIAL_COUNT],
    /// How much each material's relief (displacement map) pushes it over the others where they
    /// blend: 0 gives smooth fades, higher values crisp, natural transitions.
    pub height_blend: [f32; TERRAIN_MATERIAL_COUNT],
    pub debug_view: TerrainDebugView
}
impl TerrainRenderSettings {
    /// Takes the material parameters of `config`.
    pub fn apply_config(&mut self, config: &TerrainConfig) {
        for (i, material) in config.materials.iter().enumerate() {
            self.tiling[i] = material.tiling;
            self.height_blend[i] = material.height_blend;
        }
    }
}
impl Default for TerrainRenderSettings {
    fn default() -> Self {
        Self {
            tiling: [10.0; TERRAIN_MATERIAL_COUNT],
            height_blend: [0.5; TERRAIN_MATERIAL_COUNT],
            debug_view: TerrainDebugView::Off
        }
    }
}

/// Takes the material parameters of each spawned terrain.
pub(crate) fn apply_terrain_config(
    terrains: Query<&Terrain, Added<Terrain>>,
    mut settings: ResMut<TerrainRenderSettings>
) {
    for terrain in &terrains {
        settings.apply_config(&terrain.config);
    }
}

/// Tint of the terrain showing its masks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
pub enum TerrainDebugView {
    #[default]
    Off,
    /// Green where buildable, neutral where walkable, red where too steep to walk.
    Buildable,
    /// Slope angle, from blue (flat) to yellow (steepest walkable), then red.
    Slope
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TerrainDescription {
    pub tile_size: [f32; 3],
    pub tile_subdivisions: f32,
    pub min_height: f32,
    pub map_size: f32,
    /// 0 off, 1 buildable, 2 slope (see [`TerrainDebugView`]).
    pub debug_view: f32,
    /// Slope angle of the steepest walkable ground, over 90°.
    pub walk_max_angle: f32,
    pub tiling: [[f32; 4]; SPLAT_MAPS as usize],
    pub height_blend: [[f32; 4]; SPLAT_MAPS as usize]
}
impl TerrainDescription {
    fn new(settings: &TerrainRenderSettings, config: &TerrainConfig) -> Self {
        let pack = |values: &[f32; TERRAIN_MATERIAL_COUNT]| {
            std::array::from_fn(|k| std::array::from_fn(|c| values[k * 4 + c]))
        };
        Self {
            tile_size: [CHUNK_SIZE, config.height_range, CHUNK_SIZE],
            tile_subdivisions: CHUNK_RENDER_SUBDIVISIONS as f32,
            min_height: config.min_height,
            map_size: CHUNK_MAP_SIZE as f32,
            debug_view: match settings.debug_view {
                TerrainDebugView::Off => 0.0,
                TerrainDebugView::Buildable => 1.0,
                TerrainDebugView::Slope => 2.0
            },
            walk_max_angle: config.masks.walk_max_slope.atan() / std::f32::consts::FRAC_PI_2,
            tiling: pack(&settings.tiling),
            height_blend: pack(&settings.height_blend)
        }
    }
}

/// Per-tile GPU descriptor.
/// `tile_layer` maps this draw-call instance to its layer in the texture arrays.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TerrainTileDescription {
    pub pos: [f32; 2],
    /// Level of detail of the mesh drawing this tile.
    pub lod: f32,
    pub tile_layer: u32,
    /// Level of detail of the -X, +X, -Z and +Z neighbors: edges facing a coarser neighbor
    /// follow its edge, so that no crack opens between them.
    pub neighbor_lods: [f32; 4]
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
                    content: Some(
                        bytemuck::cast_slice(&[TerrainDescription::new(
                            &TerrainRenderSettings::default(),
                            &TerrainConfig::default()
                        )])
                        .into()
                    )
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

/// Uploads the description once the terrain is known, then whenever the settings change.
pub(crate) fn update_terrain_description_buffer(
    render_instance: Res<RenderInstance>,
    terrain_buffer: ResRenderData<TerrainBuffer>,
    buffers: Res<RenderAssets<GpuBuffer>>,
    settings: Res<TerrainRenderSettings>,
    terrain: Res<TerrainRendererGPU>,
    mut uploaded: Local<bool>
) {
    let Some(config) = &terrain.config else {
        return;
    };
    if *uploaded && !settings.is_changed() {
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
        bytemuck::cast_slice(&[TerrainDescription::new(&settings, config)]),
        0
    );
    *uploaded = true;
}

/// Uploads the chunks drawn this frame (see [`crate::render::renderer::TerrainDrawList`]).
pub(crate) fn update_terrain_tiles_buffer(
    render_instance: Res<RenderInstance>,
    terrain_buffer: ResRenderData<TerrainBuffer>,
    buffers: Res<RenderAssets<GpuBuffer>>,
    terrain: Res<TerrainRendererGPU>
) {
    if !terrain.ready || terrain.draw_list.tiles.is_empty() {
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
    let tiles = &terrain.draw_list.tiles[..terrain.draw_list.tiles.len().min(MAX_TERRAIN_TILES)];
    let render_instance = render_instance.0.read().unwrap();
    tile_buffer
        .buffer
        .write(&render_instance, bytemuck::cast_slice(tiles), 0);
}
