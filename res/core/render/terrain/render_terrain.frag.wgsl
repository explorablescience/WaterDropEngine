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

    // Compute material UVs (tiled based on vertex UVs)
    let material_uv = in.tex_coord * 25.0 % 1.0;

    // Blend albedo from each material layer
    var albedo = vec3<f32>(0.0);
    albedo += textureSample(material_albedo, material_albedo_sampler, material_uv, 0).rgb * weights.r;
    albedo += textureSample(material_albedo, material_albedo_sampler, material_uv, 1).rgb * weights.g;
    albedo += textureSample(material_albedo, material_albedo_sampler, material_uv, 2).rgb * weights.b;
    albedo += textureSample(material_albedo, material_albedo_sampler, material_uv, 3).rgb * (1.0 - weights.a);
    out.albedo_metallic = vec4<f32>(albedo, 0.0); // Terrain is non-metallic

    // Blend normal map samples (raw [0,1] values) then decode and rotate to world space
    var normal_sample = vec3<f32>(0.0);
    normal_sample += textureSample(material_normal, material_normal_sampler, material_uv, 0).rgb * weights.r;
    normal_sample += textureSample(material_normal, material_normal_sampler, material_uv, 1).rgb * weights.g;
    normal_sample += textureSample(material_normal, material_normal_sampler, material_uv, 2).rgb * weights.b;
    normal_sample += textureSample(material_normal, material_normal_sampler, material_uv, 3).rgb * (1.0 - weights.a);
    let normal_world = apply_normal_map(normal_sample, in.tangent_world.xyz, in.bitangent_world, normalize(in.normal_world));

    // Blend roughness from each material layer
    var roughness_value = 0.0;
    roughness_value += textureSample(material_roughness, material_roughness_sampler, material_uv, 0).r * weights.r;
    roughness_value += textureSample(material_roughness, material_roughness_sampler, material_uv, 1).r * weights.g;
    roughness_value += textureSample(material_roughness, material_roughness_sampler, material_uv, 2).r * weights.b;
    roughness_value += textureSample(material_roughness, material_roughness_sampler, material_uv, 3).r * (1.0 - weights.a);
    out.normal_roughness = vec4<f32>(normal_world, roughness_value);

    // Note: AO texture is sampled but not yet applied (no occlusion term in this pass)

    // Store linear view-space depth
    out.depth = in.view_z;

    return out;
}
