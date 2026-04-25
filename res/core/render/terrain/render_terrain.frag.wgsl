#include "core/render/pbr/pbr_functions.wgsl"

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,  // Clip space position (after projection)
    @location(0) tex_coord:       vec2<f32>,  // Texture coordinates (UV)
    @location(1) normal:          vec3<f32>,  // Normal in local space
    @location(2) normal_world:    vec3<f32>,  // Normal in world space
    @location(3) tangent_world:   vec4<f32>,  // Tangent in world space
    @location(4) bitangent_world: vec3<f32>,  // Bitangent in world space
    @location(5) view_z: f32                  // View-space Z (linear depth)
};

const MAX_SPLAT_LAYERS: u32 = 4u;

fn splat_weight(weights: vec4<f32>, layer: u32) -> f32 {
    if layer == 0u {
        return weights.x;
    }
    if layer == 1u {
        return weights.y;
    }
    if layer == 2u {
        return weights.z;
    }
    return 1.0 - weights.w;
}

struct TerrainDescription {
    tile_size: vec3<f32>,
    tile_subdivisions: f32,
    displacement_scales: vec4<f32>,
    tiling_scales: vec4<f32>,
}
@group(2) @binding(0) var<uniform> in_terrain_description: TerrainDescription;

// Stochastic tiling
// Eliminates visible repetition by applying a per-cell random UV
// offset and blending smoothly across the four neighbouring cells.

fn hash2(p: vec2<f32>) -> vec2<f32> {
    let q = vec2<f32>(
        dot(p, vec2<f32>(127.1, 311.7)),
        dot(p, vec2<f32>(269.5, 183.3))
    );
    return fract(sin(q) * 43758.5453123);
}

struct NoTileParams {
    uv00: vec2<f32>,
    uv10: vec2<f32>,
    uv01: vec2<f32>,
    uv11: vec2<f32>,
    w00:  f32,
    w10:  f32,
    w01:  f32,
    w11:  f32,
}

fn no_tile_params(uv: vec2<f32>) -> NoTileParams {
    let i = floor(uv);
    let f = fract(uv);
    // Quintic smooth blend so first derivative is continuous at cell edges.
    let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    return NoTileParams(
        fract(uv + hash2(i                      )),
        fract(uv + hash2(i + vec2<f32>(1.0, 0.0))),
        fract(uv + hash2(i + vec2<f32>(0.0, 1.0))),
        fract(uv + hash2(i + vec2<f32>(1.0, 1.0))),
        (1.0 - u.x) * (1.0 - u.y),
        u.x         * (1.0 - u.y),
        (1.0 - u.x) * u.y,
        u.x         * u.y
    );
}

fn no_tile_sample(
    tex:   texture_2d_array<f32>,
    samp:  sampler,
    p:     NoTileParams,
    layer: i32
) -> vec4<f32> {
    return textureSample(tex, samp, p.uv00, layer) * p.w00
         + textureSample(tex, samp, p.uv10, layer) * p.w10
         + textureSample(tex, samp, p.uv01, layer) * p.w01
         + textureSample(tex, samp, p.uv11, layer) * p.w11;
}

// Material texture arrays (group 1)
@group(1) @binding(0) var material_albedo: texture_2d_array<f32>;
@group(1) @binding(1) var material_albedo_sampler: sampler;
@group(1) @binding(2) var material_normal: texture_2d_array<f32>;
@group(1) @binding(3) var material_normal_sampler: sampler;
@group(1) @binding(4) var material_roughness: texture_2d_array<f32>;
@group(1) @binding(5) var material_roughness_sampler: sampler;
@group(1) @binding(6) var material_ao: texture_2d_array<f32>;
@group(1) @binding(7) var material_ao_sampler: sampler;

// Terrain tile data
@group(3) @binding(0) var in_heightmap: texture_2d<f32>;
@group(3) @binding(1) var in_heightmap_sampler: sampler;
@group(3) @binding(2) var in_splatmap_1: texture_2d<f32>;
@group(3) @binding(3) var in_splatmap_1_sampler: sampler;

@fragment
fn main(in: VertexOutput) -> FragOutput {
    var out: FragOutput;

    // Sample the splatmap to determine material blending weights
    let splatmap = textureSample(in_splatmap_1, in_splatmap_1_sampler, in.tex_coord);
    let weights = vec4<f32>(splatmap.r, splatmap.g, splatmap.b, splatmap.a);
    let layer_count = min(textureNumLayers(material_albedo), MAX_SPLAT_LAYERS);

    // Blend material channels from each available layer.
    var albedo = vec3<f32>(0.0);
    var normal_sample = vec3<f32>(0.0);
    var roughness_value = 0.0;
    var ao_value = 0.0;

    for (var layer: u32 = 0u; layer < layer_count; layer = layer + 1u) {
        let w = splat_weight(weights, layer);
        let material_uv = in.tex_coord * splat_weight(in_terrain_description.tiling_scales, layer);
        let p = no_tile_params(material_uv);
        let l = i32(layer);
        albedo          += no_tile_sample(material_albedo,    material_albedo_sampler,    p, l).rgb * w;
        normal_sample   += no_tile_sample(material_normal,    material_normal_sampler,    p, l).rgb * w;
        roughness_value += no_tile_sample(material_roughness, material_roughness_sampler, p, l).r   * w;
        ao_value        += no_tile_sample(material_ao,        material_ao_sampler,        p, l).r   * w;
    }

    out.albedo_metallic = vec4<f32>(albedo, 0.0); // Terrain is non-metallic

    // Blend normal map samples (raw [0,1] values) then decode and rotate to world space
    let normal_world = apply_normal_map(normal_sample, in.tangent_world.xyz, in.bitangent_world, normalize(in.normal_world));

    out.normal_roughness = vec4<f32>(normal_world, roughness_value);

    out.ao = ao_value;

    // Store linear view-space depth
    out.depth = in.view_z;

    return out;
}
