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
    return weights.w;
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

    // Compute material UVs (tiled based on vertex UVs)
    // let material_uv = in.tex_coord * 25.0 % 1.0;
    let material_uv = in.tex_coord * 2.0 % 1.0; // Allow UVs to exceed [0,1] for tiling

    // Blend material channels from each available layer.
    var albedo = vec3<f32>(0.0);
    var normal_sample = vec3<f32>(0.0);
    var roughness_value = 0.0;
    var ao_value = 0.0;

    for (var layer: u32 = 0u; layer < layer_count; layer = layer + 1u) {
        let w = splat_weight(weights, layer);
        albedo += textureSample(material_albedo, material_albedo_sampler, material_uv, i32(layer)).rgb * w;
        normal_sample += textureSample(material_normal, material_normal_sampler, material_uv, i32(layer)).rgb * w;
        roughness_value += textureSample(material_roughness, material_roughness_sampler, material_uv, i32(layer)).r * w;
        ao_value += textureSample(material_ao, material_ao_sampler, material_uv, i32(layer)).r * w;
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
