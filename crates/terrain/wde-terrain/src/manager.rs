use wde_logger::prelude::*;

use bevy::prelude::*;
use std::collections::HashMap;

use crate::{
    config::TerrainConfig,
    masks::{ChunkMasks, chunk_masks, gpu_masks},
    utils::image_decoder::decode_png
};

/// Number of chunks along each axis (terrain is CHUNK_COUNT×CHUNK_COUNT).
pub const CHUNK_COUNT: u32 = 16;
/// Number of splat-map channels packed into one RGBA texture (always 4).
pub const SPLAT_MAP_COUNT: u32 = 4;
/// Splat maps per chunk: the terrain blends `SPLAT_MAPS * SPLAT_MAP_COUNT` materials.
pub const SPLAT_MAPS: u32 = 2;

/// World-space size of one chunk.
pub const CHUNK_SIZE: f32 = 64.0;
/// Quads per chunk side at full detail.
pub const CHUNK_RENDER_SUBDIVISIONS: u32 = 256;
/// Samples per side of a chunk: its grid vertices, edges included (shared with the neighbors).
pub const CHUNK_SAMPLES: u32 = CHUNK_RENDER_SUBDIVISIONS + 1;
/// Samples per side of a chunk map on the GPU: the chunk samples, plus a one-sample border
/// copied from the neighbors (seamless normals and blending across chunks).
pub const CHUNK_MAP_SIZE: u32 = CHUNK_SAMPLES + 2;
/// Terrain loaded by default, from `res/`.
pub const DEFAULT_TERRAIN_PATH: &str = "terrain/mars";

/// 2-D integer grid position of a terrain chunk.
pub type ChunkPos = IVec2;
/// Raw bytes of one chunk map in its GPU format, `CHUNK_MAP_SIZE`² texels: `f32` normalized
/// heights for a heightmap, RGBA8 weights for a splat map, RG8 slope and buildable for the masks.
pub(crate) type TerrainTileData = Vec<u8>;
/// Kind of chunk map of a [`TerrainDirtyTile`].
pub(crate) const MAP_HEIGHT: u32 = 0;
pub(crate) const MAP_SPLAT: u32 = 1;
pub(crate) const MAP_MASKS: u32 = 2;
/// (pos, map_type [`MAP_HEIGHT`|`MAP_SPLAT`|`MAP_MASKS`], splat_index, data)
pub(crate) type TerrainDirtyTile = (ChunkPos, u32, u32, TerrainTileData);

/// Source-of-truth terrain component.
/// Owns the dirty queues consumed by the render and physics sub-systems.
///
/// Loaded from `res/<path>/`, described by its [`TerrainConfig`] (`config/terrain.json`): each
/// chunk has a heightmap (8 or 16-bit grayscale) and splat maps (RGBA, materials `4k..4k+3`),
/// at the paths the config gives. Maps of any size are resampled to the chunk samples, edges
/// included; missing heightmaps are flat at height 0.
///
/// The slope and buildable masks are derived from the heights (see [`Terrain::slope_at`]).
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Terrain {
    pub path: String,
    pub config: TerrainConfig,
    /// Maps chunk grid position → sequential index (used by UI for iteration).
    pub pos_to_tile: HashMap<ChunkPos, usize>,
    /// World heights of each chunk, `CHUNK_SAMPLES`² row-major (rows along +Z).
    #[reflect(ignore)]
    pub heights: HashMap<ChunkPos, Vec<f32>>,
    /// Masks of each chunk, same layout as the heights.
    #[reflect(ignore)]
    pub masks: HashMap<ChunkPos, ChunkMasks>,

    /// Tiles that need re-uploading to the GPU this frame.
    #[reflect(ignore)]
    pub(crate) dirty_render: Vec<TerrainDirtyTile>,
    /// Heightmap tiles that need their physics collider rebuilt.
    #[reflect(ignore)]
    pub dirty_physics: Vec<TerrainDirtyTile>
}

impl Terrain {
    /// Load terrain data from `res/<path>/`.
    pub fn load(path: &str) -> Self {
        let config = TerrainConfig::load(path);
        let full_path = Self::folder(path);
        let samples = CHUNK_SAMPLES;
        let flat = (0.0 - config.min_height) / config.height_range;

        let positions: Vec<ChunkPos> = (0..CHUNK_COUNT)
            .flat_map(|i| {
                (0..CHUNK_COUNT).map(move |j| {
                    IVec2::new(
                        i as i32 - CHUNK_COUNT as i32 / 2,
                        j as i32 - CHUNK_COUNT as i32 / 2
                    )
                })
            })
            .collect();
        let load_map = |name: &str| -> Option<_> {
            let file = format!("{full_path}/{name}");
            if std::fs::metadata(&file).is_err() {
                return None;
            }
            decode_png(&file)
                .map_err(|e| error!("Failed to load terrain map {}: {}", file, e))
                .ok()
        };

        // Decode the maps of every chunk in parallel: normalized heights, RGBA8 weights
        let load_chunk = |pos: ChunkPos| -> (ChunkPos, Vec<f32>, Vec<Vec<[u8; 4]>>) {
            let heights = match load_map(&config.heightmap_file(pos))
                .or_else(|| load_map("heightmap_default.png"))
            {
                Some(image) => image.resample(0, samples),
                None => vec![flat; (samples * samples) as usize]
            };
            let splats = (0..SPLAT_MAPS)
                .map(|k| {
                    match load_map(&config.splatmap_file(pos, k))
                        .or_else(|| load_map("splatmap_default.png"))
                    {
                        Some(image) => {
                            let channels: Vec<Vec<f32>> =
                                (0..4).map(|c| image.resample(c, samples)).collect();
                            // Images without alpha only hold three materials
                            let alpha = image.channels == 4;
                            (0..(samples * samples) as usize)
                                .map(|i| {
                                    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
                                    [
                                        byte(channels[0][i]),
                                        byte(channels[1][i]),
                                        byte(channels[2][i]),
                                        if alpha { byte(channels[3][i]) } else { 0 }
                                    ]
                                })
                                .collect()
                        }
                        // Regolith (material 1) where there is no splat map
                        None if k == 0 => vec![[0, 255, 0, 0]; (samples * samples) as usize],
                        None => vec![[0; 4]; (samples * samples) as usize]
                    }
                })
                .collect();
            (pos, heights, splats)
        };
        let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
        let loaded: Vec<_> = std::thread::scope(|scope| {
            positions
                .chunks(positions.len().div_ceil(threads))
                .map(|batch| {
                    scope.spawn(|| batch.iter().map(|p| load_chunk(*p)).collect::<Vec<_>>())
                })
                .collect::<Vec<_>>()
                .into_iter()
                .flat_map(|handle| handle.join().unwrap())
                .collect()
        });

        // Shared edges made identical between neighbors
        let mut heights: HashMap<ChunkPos, Vec<f32>> = HashMap::new();
        let mut splats: Vec<HashMap<ChunkPos, Vec<[u8; 4]>>> =
            vec![HashMap::new(); SPLAT_MAPS as usize];
        for (pos, chunk_heights, chunk_splats) in loaded {
            heights.insert(pos, chunk_heights);
            for (k, splat) in chunk_splats.into_iter().enumerate() {
                splats[k].insert(pos, splat);
            }
        }
        stitch_edges(&mut heights);

        let mut pos_to_tile = HashMap::new();
        let mut dirty_render = Vec::new();
        let mut dirty_physics = Vec::new();
        for (index, &pos) in positions.iter().enumerate() {
            let height_map: Vec<u8> = bytemuck::cast_slice(&with_border(&heights, pos)).to_vec();
            dirty_render.push((pos, MAP_HEIGHT, 0, height_map.clone()));
            dirty_physics.push((pos, MAP_HEIGHT, 0, height_map));
            for (k, maps) in splats.iter().enumerate() {
                let bytes = bytemuck::cast_slice(&with_border(maps, pos)).to_vec();
                dirty_render.push((pos, MAP_SPLAT, k as u32, bytes));
            }
            pos_to_tile.insert(pos, index);
        }

        let heights = heights
            .into_iter()
            .map(|(pos, data)| {
                let world = data
                    .iter()
                    .map(|h| config.min_height + h * config.height_range)
                    .collect();
                (pos, world)
            })
            .collect();
        let mut terrain = Self {
            path: path.to_string(),
            config,
            pos_to_tile,
            heights,
            masks: HashMap::new(),
            dirty_render,
            dirty_physics
        };
        terrain.refresh_masks(&positions);
        terrain.log_masks();
        terrain
    }

    /// Absolute path of the folder of the terrain in `res/<path>/`.
    pub fn folder(path: &str) -> String {
        let cur_dir = std::env::current_dir().unwrap();
        format!("{}/res/{}", cur_dir.display(), path)
    }

    /// Derives the masks of the given chunks from their heights (and their neighbors'), and queues
    /// them for the GPU.
    fn refresh_masks(&mut self, positions: &[ChunkPos]) {
        let rules = self.config.masks;
        let heights = &self.heights;
        let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
        let computed: Vec<(ChunkPos, ChunkMasks)> = std::thread::scope(|scope| {
            positions
                .chunks(positions.len().div_ceil(threads).max(1))
                .map(|batch| {
                    scope.spawn(move || {
                        batch
                            .iter()
                            .filter(|pos| heights.contains_key(pos))
                            .map(|pos| (*pos, chunk_masks(heights, *pos, &rules)))
                            .collect::<Vec<_>>()
                    })
                })
                .collect::<Vec<_>>()
                .into_iter()
                .flat_map(|handle| handle.join().unwrap())
                .collect()
        });
        self.masks.extend(computed);
        for (pos, bytes) in gpu_masks(&self.masks, positions) {
            self.dirty_render.push((pos, MAP_MASKS, 0, bytes));
        }
    }

    fn log_masks(&self) {
        let (mut buildable, mut steep, mut total) = (0usize, 0usize, 0usize);
        for masks in self.masks.values() {
            buildable += masks.buildable.iter().filter(|b| **b).count();
            steep += masks
                .slope
                .iter()
                .filter(|s| **s > self.config.masks.walk_max_slope)
                .count();
            total += masks.slope.len();
        }
        let percent = |count: usize| 100.0 * count as f32 / total.max(1) as f32;
        info!(
            "Loaded terrain '{}': {:.1}% buildable, {:.1}% too steep to walk.",
            self.path,
            percent(buildable),
            percent(steep)
        );
    }

    /// Returns the chunk grid position for a world position, or `None` if out of bounds.
    pub fn get_tile_idx_for_world_pos(&self, world_pos: Vec3) -> Option<IVec2> {
        let tile_pos = IVec2::new(
            (world_pos.x / CHUNK_SIZE).round() as i32,
            (world_pos.z / CHUNK_SIZE).round() as i32
        );
        self.pos_to_tile.contains_key(&tile_pos).then_some(tile_pos)
    }

    /// Terrain height at the world XZ position `pos`, or `None` outside the terrain.
    pub fn height_at(&self, pos: Vec2) -> Option<f32> {
        let chunk = (pos / CHUNK_SIZE).round().as_ivec2();
        let heights = self.heights.get(&chunk)?;
        let n = CHUNK_RENDER_SUBDIVISIONS as f32;
        let local = ((pos - chunk.as_vec2() * CHUNK_SIZE) / CHUNK_SIZE + 0.5) * n;
        let local = local.clamp(Vec2::ZERO, Vec2::splat(n));
        let (x0, z0) = (local.x.floor().min(n - 1.0), local.y.floor().min(n - 1.0));
        let (tx, tz) = (local.x - x0, local.y - z0);
        let at = |x: f32, z: f32| heights[(z as u32 * CHUNK_SAMPLES + x as u32) as usize];
        let top = at(x0, z0) * (1.0 - tx) + at(x0 + 1.0, z0) * tx;
        let bottom = at(x0, z0 + 1.0) * (1.0 - tx) + at(x0 + 1.0, z0 + 1.0) * tx;
        Some(top * (1.0 - tz) + bottom * tz)
    }

    /// Updates the CPU heights of a chunk from its GPU heightmap (e.g. after painting), and the
    /// masks around it.
    pub fn set_chunk_heights(&mut self, pos: ChunkPos, map: &[u8]) {
        let (min, range) = (self.config.min_height, self.config.height_range);
        let world = chunk_samples(map)
            .into_iter()
            .map(|h| min + h * range)
            .collect();
        self.heights.insert(pos, world);
        // The build margin reaches into the neighbors
        let around: Vec<ChunkPos> = (-1..=1)
            .flat_map(|z| (-1..=1).map(move |x| pos + IVec2::new(x, z)))
            .filter(|p| self.heights.contains_key(p))
            .collect();
        self.refresh_masks(&around);
    }
}

/// Normalized chunk samples of a GPU heightmap (without its border).
pub fn chunk_samples(map: &[u8]) -> Vec<f32> {
    let values: &[f32] = bytemuck::cast_slice(map);
    let size = CHUNK_MAP_SIZE as usize;
    (1..size - 1)
        .flat_map(|z| (1..size - 1).map(move |x| values[z * size + x]))
        .collect()
}

/// Averages the samples shared by neighbor chunks, so that their edges match exactly.
fn stitch_edges(maps: &mut HashMap<ChunkPos, Vec<f32>>) {
    let n = CHUNK_SAMPLES as usize;
    let last = n - 1;
    let positions: Vec<ChunkPos> = maps.keys().copied().collect();
    for &pos in &positions {
        for (offset, vertical) in [(IVec2::X, true), (IVec2::Y, false)] {
            let other = pos + offset;
            if !maps.contains_key(&other) {
                continue;
            }
            for k in 0..n {
                // East edge (last column) with the west edge (first column), or north with south
                let (a, b) = match vertical {
                    true => (k * n + last, k * n),
                    false => (last * n + k, k)
                };
                let mean = (maps[&pos][a] + maps[&other][b]) * 0.5;
                maps.get_mut(&pos).unwrap()[a] = mean;
                maps.get_mut(&other).unwrap()[b] = mean;
            }
        }
    }
    // Corners shared by four chunks
    for &pos in &positions {
        let corners = [
            (pos, last * n + last),
            (pos + IVec2::X, last * n),
            (pos + IVec2::Y, last),
            (pos + IVec2::ONE, 0)
        ];
        let present: Vec<_> = corners
            .iter()
            .filter(|(p, _)| maps.contains_key(p))
            .collect();
        let mean = present.iter().map(|(p, i)| maps[p][*i]).sum::<f32>() / present.len() as f32;
        for (p, i) in present {
            maps.get_mut(p).unwrap()[*i] = mean;
        }
    }
}

/// A chunk map with its one-sample border, taken from the neighbors (or extended at the edges
/// of the terrain).
pub(crate) fn with_border<T: Copy>(maps: &HashMap<ChunkPos, Vec<T>>, pos: ChunkPos) -> Vec<T> {
    with_margin(maps, pos, 1)
}

/// A chunk map extended by `margin` samples on each side (`margin` < `CHUNK_SAMPLES`), taken
/// from the neighbors (or extended at the edges of the terrain), row-major.
pub(crate) fn with_margin<T: Copy>(
    maps: &HashMap<ChunkPos, Vec<T>>,
    pos: ChunkPos,
    margin: i32
) -> Vec<T> {
    let n = CHUNK_SAMPLES as i32;
    let own = &maps[&pos];
    let neighbors: [[Option<&Vec<T>>; 3]; 3] = std::array::from_fn(|z| {
        std::array::from_fn(|x| maps.get(&(pos + IVec2::new(x as i32 - 1, z as i32 - 1))))
    });
    // Beyond an edge, the neighbor's samples past the shared edge
    let split = |v: i32| match v {
        v if v < 0 => (0, v + n - 1),
        v if v >= n => (2, v - (n - 1)),
        v => (1, v)
    };
    let size = n + 2 * margin;
    let mut out = Vec::with_capacity((size * size) as usize);
    for z in -margin..n + margin {
        let (cz, lz) = split(z);
        for x in -margin..n + margin {
            let (cx, lx) = split(x);
            out.push(match neighbors[cz][cx] {
                Some(map) => map[(lz * n + lx) as usize],
                None => own[(z.clamp(0, n - 1) * n + x.clamp(0, n - 1)) as usize]
            });
        }
    }
    out
}
