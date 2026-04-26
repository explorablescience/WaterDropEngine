#include "core/render/pbr/pbr_functions.wgsl"
#include "core/render/terrain/terrain_common.wgsl"

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,  // Clip space position (after projection)
    @location(0) tex_coord:       vec2<f32>,  // Texture coordinates (UV)
    @location(1) normal:          vec3<f32>,  // Normal in local space
    @location(2) normal_world:    vec3<f32>,  // Normal in world space
    @location(3) tangent_world:   vec4<f32>,  // Tangent in world space
    @location(4) bitangent_world: vec3<f32>,  // Bitangent in world space
    @location(5) view_z: f32,                 // View-space Z (linear depth)
    @location(6) tile_pos: vec2<f32>          // Terrain tile index in grid space
};

@group(2) @binding(0) var<uniform> in_terrain_description: TerrainDescription;

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
    let layer_count = terrain_layer_count(textureNumLayers(material_albedo));

    // Blend material channels from each available layer.
    var albedo = vec3<f32>(0.0);
    var normal_sample = vec3<f32>(0.0);
    var roughness_value = 0.0;
    var ao_value = 0.0;

    for (var layer: u32 = 0u; layer < layer_count; layer = layer + 1u) {
        let uv = get_terrain_uv(in.tex_coord, in.tile_pos, in_terrain_description, layer);
        let w = splat_weight(weights, layer);
        albedo          += w * textureSample(material_albedo, material_albedo_sampler, uv, layer).rgb;
        normal_sample   += w * textureSample(material_normal, material_normal_sampler, uv, layer).rgb;
        roughness_value += w * textureSample(material_roughness, material_roughness_sampler, uv, layer).r;
        ao_value        += w * textureSample(material_ao, material_ao_sampler, uv, layer).r;
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
