use std::collections::HashMap;

use bevy::{ecs::system::SystemParamItem, prelude::*};
use wde_renderer::prelude::*;

use crate::{
    config::TerrainConfig,
    manager::{ChunkPos, MAP_HEIGHT, MAP_MASKS, MAP_SPLAT, TerrainDirtyTile},
    prelude::TerrainRenderer,
    render::renderer::TerrainDrawList
};

// ─── Render bind group (texture_2d_array sampling) ──────────────────────────

/// Heightmaps (read texel by texel, unfiltered), then each splat map array with its sampler,
/// then the masks with theirs.
#[derive(Asset, Clone, TypePath, Default)]
pub struct TerrainChunkArrayBg {
    pub heightmap_array: Option<Handle<Texture>>,
    pub splatmap_arrays: Vec<Handle<Texture>>,
    pub mask_array: Option<Handle<Texture>>
}
impl RenderBinding for TerrainChunkArrayBg {
    type Params = ();

    fn describe(
        &mut self,
        _params: &SystemParamItem<Self::Params>,
        builder: &mut RenderBindingBuilder
    ) {
        builder.add_texture_array_view_from_id(self.heightmap_array.as_ref().map(|h| h.id()));
        for splatmap in &self.splatmap_arrays {
            builder
                .add_texture_array_view_from_id(Some(splatmap.id()))
                .add_texture_sampler_from_id(Some(splatmap.id()));
        }
        let mask = self.mask_array.as_ref().map(|h| h.id());
        builder
            .add_texture_array_view_from_id(mask)
            .add_texture_sampler_from_id(mask);
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
    /// Chunks to draw this frame.
    pub draw_list: TerrainDrawList,

    pub heightmap_array: Option<Handle<Texture>>,
    pub splatmap_arrays: Vec<Handle<Texture>>,
    pub mask_array: Option<Handle<Texture>>,
    /// Description of the terrain drawn, set with the texture handles.
    pub config: Option<TerrainConfig>,

    pub render_bind_group: Option<Handle<TerrainChunkArrayBg>>,
    pub compute_bind_group: Option<Handle<TerrainComputeArrayBg>>,

    pub dirty: Vec<TerrainDirtyTile>
}

impl TerrainRendererGPU {
    /// Transfer the dirty tile uploads and the chunks to draw from the main world renderer.
    /// Also does the one-time GPU handle initialisation on the first call.
    pub fn extract_renderer(&mut self, renderer: &TerrainRenderer) {
        if self.heightmap_array.is_none() {
            self.heightmap_array = renderer.heightmap_array.clone();
            self.splatmap_arrays = renderer.splatmap_arrays.clone();
            self.mask_array = renderer.mask_array.clone();
            self.pos_to_layer = renderer.pos_to_layer.clone();
            self.config = Some(renderer.config.clone());
        }
        self.dirty.extend(renderer.dirty.iter().cloned());
        self.draw_list = renderer.draw_list.clone();
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
        for (pos, map_type, splat_index, tile_data) in dirty {
            let layer = match gpu.pos_to_layer.get(&pos) {
                Some(&l) => l,
                None => continue
            };
            let handle = match map_type {
                MAP_HEIGHT => gpu.heightmap_array.as_ref(),
                MAP_SPLAT => gpu.splatmap_arrays.get(splat_index as usize),
                MAP_MASKS => gpu.mask_array.as_ref(),
                _ => unreachable!()
            };
            if let Some(tex) = handle.and_then(|h| textures.get(h)) {
                tex.texture.copy_from_buffer_layered(
                    &render_instance,
                    tex.texture.format,
                    layer,
                    bytemuck::cast_slice(&tile_data)
                );
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
        if gpu.heightmap_array.is_none()
            || gpu.splatmap_arrays.is_empty()
            || gpu.mask_array.is_none()
        {
            return;
        }

        if gpu.render_bind_group.is_none() {
            gpu.render_bind_group = Some(asset_server.add(TerrainChunkArrayBg {
                heightmap_array: gpu.heightmap_array.clone(),
                splatmap_arrays: gpu.splatmap_arrays.clone(),
                mask_array: gpu.mask_array.clone()
            }));
        }
        // The terrain editor paints the first splat map (materials 0 to 3)
        if gpu.compute_bind_group.is_none() {
            gpu.compute_bind_group = Some(asset_server.add(TerrainComputeArrayBg {
                heightmap_array: gpu.heightmap_array.clone(),
                splatmap_array: gpu.splatmap_arrays.first().cloned()
            }));
        }

        gpu.ready = gpu.render_bind_group.is_some() && gpu.compute_bind_group.is_some();
    }
}
