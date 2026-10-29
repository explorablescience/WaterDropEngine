use std::collections::HashMap;

use bevy::prelude::*;

use crate::{
    config::TerrainMaskRules,
    manager::{
        CHUNK_COUNT, CHUNK_RENDER_SUBDIVISIONS, CHUNK_SAMPLES, CHUNK_SIZE, ChunkPos, Terrain,
        with_border, with_margin
    }
};

/// World distance between two samples.
const SAMPLE_STEP: f32 = CHUNK_SIZE / CHUNK_RENDER_SUBDIVISIONS as f32;

/// Masks of one chunk, derived from the heights: `CHUNK_SAMPLES`² row-major (rows along +Z).
#[derive(Clone, Debug, Default)]
pub struct ChunkMasks {
    /// Ground slope (height change per unit of distance).
    pub slope: Vec<f32>,
    /// Flat ground at height 0, away from any ground that is not.
    pub buildable: Vec<bool>
}

/// Derives the masks of a chunk from the heights of the chunk and its neighbors.
pub(crate) fn chunk_masks(
    heights: &HashMap<ChunkPos, Vec<f32>>,
    pos: ChunkPos,
    rules: &TerrainMaskRules
) -> ChunkMasks {
    let n = CHUNK_SAMPLES as i32;
    let margin = (rules.build_margin / SAMPLE_STEP).ceil().max(0.0) as i32;
    // Heights one sample beyond the margin, for the slopes over the margin
    let r = margin + 1;
    let size = n + 2 * r;
    let window = with_margin(heights, pos, r);
    let height = |x: i32, z: i32| window[((z + r) * size + x + r) as usize];
    let slope = |x: i32, z: i32| {
        let gx = height(x + 1, z) - height(x - 1, z);
        let gz = height(x, z + 1) - height(x, z - 1);
        gx.hypot(gz) / (2.0 * SAMPLE_STEP)
    };

    // Flat ground at height 0 over the chunk and its margin
    let m = n + 2 * margin;
    let mut flat = Vec::with_capacity((m * m) as usize);
    for z in -margin..n + margin {
        for x in -margin..n + margin {
            flat.push(
                height(x, z).abs() <= rules.build_height_tolerance
                    && slope(x, z) <= rules.build_max_slope
            );
        }
    }

    // Buildable where the whole square of `margin` around is flat (separable erosion)
    let (m, n) = (m as usize, n as usize);
    let span = 2 * margin as usize;
    let rows: Vec<bool> = (0..m)
        .flat_map(|z| (0..n).map(move |x| (z, x)))
        .map(|(z, x)| flat[z * m + x..=z * m + x + span].iter().all(|f| *f))
        .collect();
    let buildable = (0..n)
        .flat_map(|z| (0..n).map(move |x| (z, x)))
        .map(|(z, x)| (z..=z + span).all(|zi| rows[zi * n + x]))
        .collect();

    let slope = (0..n as i32)
        .flat_map(|z| (0..n as i32).map(move |x| (x, z)))
        .map(|(x, z)| slope(x, z))
        .collect();
    ChunkMasks { slope, buildable }
}

/// GPU masks of the given chunks, with their border: RG8 texels holding the slope angle over 90°
/// and whether the ground is buildable.
pub(crate) fn gpu_masks(
    masks: &HashMap<ChunkPos, ChunkMasks>,
    positions: &[ChunkPos]
) -> Vec<(ChunkPos, Vec<u8>)> {
    // The chunks and the neighbors their border is taken from
    let encoded: HashMap<ChunkPos, Vec<[u8; 2]>> = positions
        .iter()
        .flat_map(|pos| (-1..=1).flat_map(move |z| (-1..=1).map(move |x| *pos + IVec2::new(x, z))))
        .filter_map(|pos| {
            let chunk = masks.get(&pos)?;
            let texels = chunk
                .slope
                .iter()
                .zip(&chunk.buildable)
                .map(|(slope, buildable)| {
                    let angle = slope.atan() / std::f32::consts::FRAC_PI_2;
                    [(angle * 255.0).round() as u8, *buildable as u8 * 255]
                })
                .collect();
            Some((pos, texels))
        })
        .collect();
    positions
        .iter()
        .filter(|pos| encoded.contains_key(pos))
        .map(|pos| {
            (
                *pos,
                bytemuck::cast_slice(&with_border(&encoded, *pos)).to_vec()
            )
        })
        .collect()
}

impl Terrain {
    /// Ground slope at the world XZ position `pos` (height change per unit of distance, at the
    /// nearest sample), or `None` outside the terrain.
    pub fn slope_at(&self, pos: Vec2) -> Option<f32> {
        let (chunk, index) = Self::sample_index(Self::nearest_sample(pos))?;
        self.masks.get(&chunk).map(|m| m.slope[index])
    }

    /// Whether the ground at `pos` is gentle enough to walk on.
    pub fn is_walkable(&self, pos: Vec2) -> bool {
        self.slope_at(pos)
            .is_some_and(|slope| slope <= self.config.masks.walk_max_slope)
    }

    /// Whether the ground at `pos` is buildable (at the nearest sample).
    pub fn is_buildable(&self, pos: Vec2) -> bool {
        Self::sample_index(Self::nearest_sample(pos))
            .and_then(|(chunk, index)| Some(self.masks.get(&chunk)?.buildable[index]))
            .unwrap_or(false)
    }

    /// Whether the ground is buildable at every sample of the world XZ rectangle `min`..`max`.
    pub fn is_area_buildable(&self, min: Vec2, max: Vec2) -> bool {
        let origin = Self::sample_origin();
        let from = ((min - origin) / SAMPLE_STEP - 1e-3).ceil().as_ivec2();
        let to = ((max - origin) / SAMPLE_STEP + 1e-3).floor().as_ivec2();
        // Smaller than a sample: its nearest one
        let center = Self::nearest_sample((min + max) * 0.5);
        let (from, to) = (from.min(center), to.max(center));
        (from.y..=to.y).all(|z| {
            (from.x..=to.x).all(|x| {
                Self::sample_index(IVec2::new(x, z))
                    .and_then(|(chunk, index)| Some(self.masks.get(&chunk)?.buildable[index]))
                    .unwrap_or(false)
            })
        })
    }

    /// World XZ position of the first sample of the terrain.
    fn sample_origin() -> Vec2 {
        Vec2::splat(-((CHUNK_COUNT / 2) as f32) * CHUNK_SIZE - CHUNK_SIZE / 2.0)
    }

    /// Global sample nearest to `pos`, counted from the first sample of the terrain.
    fn nearest_sample(pos: Vec2) -> IVec2 {
        ((pos - Self::sample_origin()) / SAMPLE_STEP)
            .round()
            .as_ivec2()
    }

    /// Chunk and index in it of a global sample, `None` outside the terrain. Samples shared by
    /// neighbor chunks are taken from the first one.
    fn sample_index(sample: IVec2) -> Option<(ChunkPos, usize)> {
        let res = CHUNK_RENDER_SUBDIVISIONS as i32;
        let last = CHUNK_COUNT as i32 * res;
        if sample.min_element() < 0 || sample.max_element() > last {
            return None;
        }
        let chunk = (sample / res).min(IVec2::splat(CHUNK_COUNT as i32 - 1));
        let local = sample - chunk * res;
        Some((
            chunk - IVec2::splat(CHUNK_COUNT as i32 / 2),
            (local.y * CHUNK_SAMPLES as i32 + local.x) as usize
        ))
    }
}
