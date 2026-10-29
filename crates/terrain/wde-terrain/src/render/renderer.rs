use std::collections::HashMap;

use bevy::{prelude::*, window::PrimaryWindow};
use wde_camera::prelude::*;
use wde_renderer::prelude::*;

use crate::{
    config::TerrainConfig,
    manager::{
        CHUNK_COUNT, CHUNK_MAP_SIZE, CHUNK_SIZE, ChunkPos, MAP_HEIGHT, SPLAT_MAPS, Terrain,
        TerrainDirtyTile
    },
    render::dependencies::terrain_buffer::TerrainTileDescription
};

/// Number of mesh levels of detail: level `k` has `CHUNK_RENDER_SUBDIVISIONS >> k` quads per side.
pub const TERRAIN_LOD_COUNT: usize = 5;
/// Distance from the camera within which chunks are drawn at full detail; each next level of
/// detail starts twice as far.
const LOD_BASE_DISTANCE: f32 = 48.0;

/// Chunks to draw this frame, sorted by level of detail.
#[derive(Clone, Debug, Default)]
pub struct TerrainDrawList {
    pub tiles: Vec<TerrainTileDescription>,
    /// Range of `tiles` drawn with each level of detail mesh.
    pub lod_ranges: [std::ops::Range<u32>; TERRAIN_LOD_COUNT]
}

/// CPU-side terrain renderer.
/// Holds the shared texture arrays (one heightmap array, `SPLAT_MAPS` splat map arrays, one mask
/// array), the dirty queue forwarded to the GPU each frame, and the chunks to draw.
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct TerrainRenderer {
    /// Description of the terrain drawn.
    pub config: TerrainConfig,
    /// Maps chunk position → array layer index (= tile index).
    pub pos_to_layer: HashMap<ChunkPos, u32>,
    /// Single R32Float texture array (normalized heights): one layer per chunk.
    #[reflect(ignore)]
    pub heightmap_array: Option<Handle<Texture>>,
    /// Rgba8Unorm texture arrays of the materials weights: one layer per chunk.
    #[reflect(ignore)]
    pub splatmap_arrays: Vec<Handle<Texture>>,
    /// Rg8Unorm texture array of the slope and buildable masks: one layer per chunk.
    #[reflect(ignore)]
    pub mask_array: Option<Handle<Texture>>,
    /// Dirty tiles forwarded from `Terrain` each frame.
    #[reflect(ignore)]
    pub dirty: Vec<TerrainDirtyTile>,
    /// Chunks to draw this frame.
    #[reflect(ignore)]
    pub draw_list: TerrainDrawList,
    /// Lowest and highest world height of each chunk, for culling.
    #[reflect(ignore)]
    bounds: HashMap<ChunkPos, (f32, f32)>
}

impl TerrainRenderer {
    /// Create the shared texture arrays, one layer per chunk of the terrain.
    pub fn new(asset_server: &AssetServer, terrain: &Terrain) -> Self {
        let layer_count = CHUNK_COUNT * CHUNK_COUNT;
        let usages = TextureUsages::COPY_SRC
            | TextureUsages::COPY_DST
            | TextureUsages::TEXTURE_BINDING
            | TextureUsages::STORAGE_BINDING;

        let heightmap_array = asset_server.add(Texture {
            label: "terrain_heightmap_array".to_string(),
            size: (CHUNK_MAP_SIZE, CHUNK_MAP_SIZE),
            format: TextureFormat::R32Float,
            usages,
            layer_count,
            filterable: false,
            ..Default::default()
        });
        let splatmap_arrays = (0..SPLAT_MAPS)
            .map(|k| {
                asset_server.add(Texture {
                    label: format!("terrain_splatmap_array_{k}"),
                    size: (CHUNK_MAP_SIZE, CHUNK_MAP_SIZE),
                    format: TextureFormat::Rgba8Unorm,
                    usages,
                    layer_count,
                    ..Default::default()
                })
            })
            .collect();
        let mask_array = asset_server.add(Texture {
            label: "terrain_mask_array".to_string(),
            size: (CHUNK_MAP_SIZE, CHUNK_MAP_SIZE),
            format: TextureFormat::Rg8Unorm,
            usages: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
            layer_count,
            ..Default::default()
        });

        let pos_to_layer = terrain
            .pos_to_tile
            .iter()
            .map(|(pos, tile)| (*pos, *tile as u32))
            .collect();

        Self {
            config: terrain.config.clone(),
            pos_to_layer,
            heightmap_array: Some(heightmap_array),
            splatmap_arrays,
            mask_array: Some(mask_array),
            dirty: Vec::new(),
            draw_list: TerrainDrawList::default(),
            bounds: HashMap::new()
        }
    }

    /// Move dirty tiles from `Terrain` into this renderer's queue.
    pub fn extract_dirty(
        mut renderer: Query<&mut TerrainRenderer>,
        mut terrain: Query<&mut Terrain>
    ) {
        let (Ok(mut renderer), Ok(mut terrain)) = (renderer.single_mut(), terrain.single_mut())
        else {
            return;
        };
        renderer.dirty.clear();
        renderer.dirty.append(&mut terrain.dirty_render);

        // Height bounds of the chunks whose heightmap changed
        let renderer = renderer.as_mut();
        for (pos, map_type, _, _) in &renderer.dirty {
            if *map_type == MAP_HEIGHT
                && let Some(heights) = terrain.heights.get(pos)
            {
                let bounds = heights
                    .iter()
                    .fold((f32::MAX, f32::MIN), |(lo, hi), v| (lo.min(*v), hi.max(*v)));
                renderer.bounds.insert(*pos, bounds);
            }
        }
    }

    /// Chooses the chunks to draw, outside of the [`CullingCamera`] frustum excluded, and their
    /// level of detail from their distance to it.
    pub fn update_draw_list(
        mut renderer: Query<&mut TerrainRenderer>,
        camera: Query<(&GlobalTransform, &CameraView), With<CullingCamera>>,
        window: Query<&Window, With<PrimaryWindow>>
    ) {
        let (Ok(mut renderer), Ok((camera_transform, view))) =
            (renderer.single_mut(), camera.single())
        else {
            return;
        };
        let aspect = window
            .single()
            .map_or(1.0, |w| w.width() / w.height().max(1.0));
        let frustum = Frustum::from_view_proj(view.world_to_ndc(camera_transform, aspect));
        let eye = camera_transform.translation();

        let renderer = renderer.as_mut();
        let half = CHUNK_SIZE / 2.0;
        let lod_of = |pos: ChunkPos| -> Option<u32> {
            let (lo, hi) = *renderer.bounds.get(&pos)?;
            let center = pos.as_vec2() * CHUNK_SIZE;
            let min = Vec3::new(center.x - half, lo, center.y - half);
            let max = Vec3::new(center.x + half, hi, center.y + half);
            let distance = eye.clamp(min, max).distance(eye);
            let lod = (distance / LOD_BASE_DISTANCE).max(1.0).log2().floor() as u32 + 1;
            Some(match distance < LOD_BASE_DISTANCE {
                true => 0,
                false => lod.min(TERRAIN_LOD_COUNT as u32 - 1)
            })
        };
        let visible = |pos: ChunkPos| -> bool {
            let Some(&(lo, hi)) = renderer.bounds.get(&pos) else {
                return false;
            };
            let center = pos.as_vec2() * CHUNK_SIZE;
            frustum.intersects(&Aabb::new(
                Vec3::new(center.x - half, lo, center.y - half),
                Vec3::new(center.x + half, hi, center.y + half)
            ))
        };

        let lods: HashMap<ChunkPos, u32> = renderer
            .pos_to_layer
            .keys()
            .filter_map(|pos| Some((*pos, lod_of(*pos)?)))
            .collect();
        let mut tiles: Vec<TerrainTileDescription> = renderer
            .pos_to_layer
            .iter()
            .filter(|(pos, _)| visible(**pos))
            .filter_map(|(pos, layer)| {
                let lod = *lods.get(pos)?;
                // Neighbors absent from the terrain do not constrain the edges
                let neighbor = |offset: IVec2| *lods.get(&(*pos + offset)).unwrap_or(&lod) as f32;
                Some(TerrainTileDescription {
                    pos: [pos.x as f32, pos.y as f32],
                    lod: lod as f32,
                    tile_layer: *layer,
                    neighbor_lods: [
                        neighbor(IVec2::NEG_X),
                        neighbor(IVec2::X),
                        neighbor(IVec2::NEG_Y),
                        neighbor(IVec2::Y)
                    ]
                })
            })
            .collect();
        tiles.sort_by_key(|t| t.lod as u32);

        let mut lod_ranges: [std::ops::Range<u32>; TERRAIN_LOD_COUNT] = Default::default();
        for (lod, range) in lod_ranges.iter_mut().enumerate() {
            let start = tiles.partition_point(|t| (t.lod as usize) < lod) as u32;
            let end = tiles.partition_point(|t| (t.lod as usize) <= lod) as u32;
            *range = start..end;
        }
        renderer.draw_list = TerrainDrawList { tiles, lod_ranges };
    }
}
