#include "core/render/pbr/pbr_functions.wgsl"
#include "core/render/terrain/terrain_common.wgsl"

struct ModelInput {
    @location(0) position:  vec3<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) normal:    vec3<f32>,
    @location(3) tangent:   vec4<f32>
};
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
    @location(1) world_pos: vec3<f32>,
    @location(2) ndc_z:     f32,
    @location(3) @interpolate(flat) tile_layer: u32
};

@group(0) @binding(0) var<uniform> in_camera: Camera;

@group(2) @binding(0) var<uniform> in_terrain: TerrainDescription;
@group(2) @binding(1) var<storage, read> in_tiles: array<TerrainTile>;

@group(3) @binding(0) var in_heightmap: texture_2d_array<f32>;

// Normalized height of chunk sample (i, j), in [0, n]; the border extends one sample beyond.
fn sample_height(i: i32, j: i32, layer: i32) -> f32 {
    return textureLoad(in_heightmap, vec2<i32>(i + 1, j + 1), layer, 0).r;
}

// Height of an edge vertex facing a coarser neighbor: on the neighbor's edge, between its vertices.
fn coarse_edge_height(i: i32, j: i32, along_i: bool, step: i32, n: i32, layer: i32) -> f32 {
    if along_i {
        let i0 = (i / step) * step;
        let i1 = min(i0 + step, n);
        return mix(sample_height(i0, j, layer), sample_height(i1, j, layer), f32(i - i0) / f32(step));
    }
    let j0 = (j / step) * step;
    let j1 = min(j0 + step, n);
    return mix(sample_height(i, j0, layer), sample_height(i, j1, layer), f32(j - j0) / f32(step));
}

@vertex
fn main(@builtin(instance_index) instance: u32, model: ModelInput) -> VertexOutput {
    var out: VertexOutput;
    let tile  = in_tiles[instance];
    let layer = i32(tile.tile_layer);

    // Vertices land exactly on heightmap samples, whatever the level of detail
    let n = i32(in_terrain.tile_subdivisions);
    let i = i32(round(model.tex_coord.x * f32(n)));
    let j = i32(round(model.tex_coord.y * f32(n)));
    var h = sample_height(i, j, layer);

    // Edges follow coarser neighbors, so that no crack opens between levels of detail
    let lod = tile.lod;
    let nl  = tile.neighbor_lods;
    if i == 0 && nl.x > lod {
        h = coarse_edge_height(i, j, false, 1 << u32(nl.x), n, layer);
    }
    if i == n && nl.y > lod {
        h = coarse_edge_height(i, j, false, 1 << u32(nl.y), n, layer);
    }
    if j == 0 && nl.z > lod {
        h = coarse_edge_height(i, j, true, 1 << u32(nl.z), n, layer);
    }
    if j == n && nl.w > lod {
        h = coarse_edge_height(i, j, true, 1 << u32(nl.w), n, layer);
    }

    // Slightly below the true ground, so that flat building foundations never z-fight with it
    let offset = tile.pos * in_terrain.tile_size.xz;
    let world_pos = vec3<f32>(
        model.position.x + offset.x,
        in_terrain.min_height + h * in_terrain.tile_size.y - 0.02,
        model.position.z + offset.y
    );
    let view_pos = in_camera.world_to_view * vec4<f32>(world_pos, 1.0);
    out.clip_position = in_camera.view_to_ndc * view_pos;
    out.ndc_z      = -view_pos.z;
    out.tex_coord  = model.tex_coord;
    out.world_pos  = world_pos;
    out.tile_layer = tile.tile_layer;
    return out;
}
