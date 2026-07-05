use std::collections::HashMap;

use bevy::{ecs::system::SystemParamItem, prelude::*};
use wde_renderer::prelude::*;

use crate::{
    manager::{ChunkPos, TerrainDirtyTile},
    prelude::TerrainExtractor,
    render::extractor
};

// ─── Render bind group (texture_2d_array sampling) ──────────────────────────

#[derive(Asset, Clone, TypePath, Default)]
pub struct TerrainChunkArrayBg {
    pub heightmap_array: Option<Handle<Texture>>,
    pub splatmap_array: Option<Handle<Texture>>
}
impl RenderBinding for TerrainChunkArrayBg {
    type Params = ();

    fn describe(
        &mut self,
        _params: &SystemParamItem<Self::Params>,
        builder: &mut RenderBindingBuilder
    ) {
        builder
            .add_texture_array_view_from_id(self.heightmap_array.as_ref().map(|h| h.id()))
            .add_texture_sampler_from_id(self.heightmap_array.as_ref().map(|h| h.id()))
            .add_texture_array_view_from_id(self.splatmap_array.as_ref().map(|h| h.id()))
            .add_texture_sampler_from_id(self.splatmap_array.as_ref().map(|h| h.id()));
    }

    fn label(&self) -> &str {
        "terrain-chunk-array-render"
    }
}

// ─── Compute bind group (texture_storage_2d_array read/write) ────────────────

#[derive(Asset, Clone, TypePath, Default)]
pub struct TerrainComputeArrayBg {
    pub heightmap_array: Option<Handle<Texture>>,
    pub splatmap_array: Option<Handle<Texture>>
}
impl RenderBinding for TerrainComputeArrayBg {
    type Params = ();

    fn describe(
        &mut self,
        _params: &SystemParamItem<Self::Params>,
        builder: &mut RenderBindingBuilder
    ) {
        builder
            .add_storage_texture_array_from_id(self.heightmap_array.as_ref().map(|h| h.id()))
            .add_storage_texture_array_from_id(self.splatmap_array.as_ref().map(|h| h.id()));
    }

    fn label(&self) -> &str {
        "terrain-chunk-array-compute"
    }
}

// ─── GPU-side renderer resource ──────────────────────────────────────────────

#[derive(Resource, Default)]
pub struct TerrainRendererGPU {
    /// True once bind groups are created and the renderer is ready.
    pub ready: bool,
    /// Maps chunk position → array layer index.
    pub pos_to_layer: HashMap<ChunkPos, u32>,
    /// Total number of chunks to draw.
    pub chunk_count: u32,

    pub heightmap_array: Option<Handle<Texture>>,
    pub splatmap_array: Option<Handle<Texture>>,

    pub render_bind_group: Option<Handle<TerrainChunkArrayBg>>,
    pub compute_bind_group: Option<Handle<TerrainComputeArrayBg>>,

    pub dirty: Vec<TerrainDirtyTile>
}

impl TerrainRendererGPU {
    /// Transfer the physics extraction queue and dirty tile uploads from the main world.
    /// Also does the one-time GPU handle initialisation on the first call.
    #[allow(clippy::too_many_arguments)]
    pub fn extract_dirty(
        main_terrain_extractor: &mut TerrainExtractor,
        render_terrain_extractor: &mut TerrainExtractor,
        heightmap_array: Option<Handle<Texture>>,
        splatmap_array: Option<Handle<Texture>>,
        pos_to_layer: HashMap<ChunkPos, u32>,
        dirty: Vec<TerrainDirtyTile>,
        terrain_renderer_chunk_count: u32,
        gpu_terrain_renderer: &mut TerrainRendererGPU
    ) {
        // Move the tile-extraction queue from main world → render world.
        extractor::extract_dirty(main_terrain_extractor, render_terrain_extractor);

        // One-time initialisation of GPU handles.
        if gpu_terrain_renderer.heightmap_array.is_none() {
            gpu_terrain_renderer.heightmap_array = heightmap_array;
            gpu_terrain_renderer.splatmap_array = splatmap_array;
            gpu_terrain_renderer.pos_to_layer = pos_to_layer;
            gpu_terrain_renderer.chunk_count = terrain_renderer_chunk_count;
        }

        gpu_terrain_renderer.dirty.extend(dirty);
    }

    /// Upload dirty tile data to the appropriate array layer on the GPU.
    pub fn upload_dirty(
        mut gpu: ResMut<TerrainRendererGPU>,
        textures: Res<RenderAssets<GpuTexture>>,
        render_instance: Res<RenderInstance>
    ) {
        if gpu.dirty.is_empty() {
            return;
        }
        let render_instance = render_instance.0.read().unwrap();
        let dirty = std::mem::take(&mut gpu.dirty);
        for (pos, map_type, _, tile_data) in dirty {
            let layer = match gpu.pos_to_layer.get(&pos) {
                Some(&l) => l,
                None => continue
            };
            match map_type {
                0 => {
                    if let Some(handle) = &gpu.heightmap_array
                        && let Some(tex) = textures.get(handle)
                    {
                        tex.texture.copy_from_buffer_layered(
                            &render_instance,
                            tex.texture.format,
                            layer,
                            bytemuck::cast_slice(&tile_data)
                        );
                    }
                }
                1 => {
                    if let Some(handle) = &gpu.splatmap_array
                        && let Some(tex) = textures.get(handle)
                    {
                        tex.texture.copy_from_buffer_layered(
                            &render_instance,
                            tex.texture.format,
                            layer,
                            bytemuck::cast_slice(&tile_data)
                        );
                    }
                }
                _ => unreachable!()
            }
        }
    }

    /// Create the single render and compute bind groups once textures are ready.
    pub fn prepare_bind_groups(
        asset_server: Res<AssetServer>,
        mut gpu: ResMut<TerrainRendererGPU>
    ) {
        if gpu.ready {
            return;
        }
        if gpu.heightmap_array.is_none() || gpu.splatmap_array.is_none() {
            return;
        }

        if gpu.render_bind_group.is_none() {
            gpu.render_bind_group = Some(asset_server.add(TerrainChunkArrayBg {
                heightmap_array: gpu.heightmap_array.clone(),
                splatmap_array: gpu.splatmap_array.clone()
            }));
        }
        if gpu.compute_bind_group.is_none() {
            gpu.compute_bind_group = Some(asset_server.add(TerrainComputeArrayBg {
                heightmap_array: gpu.heightmap_array.clone(),
                splatmap_array: gpu.splatmap_array.clone()
            }));
        }

        gpu.ready = gpu.render_bind_group.is_some() && gpu.compute_bind_group.is_some();
    }
}
