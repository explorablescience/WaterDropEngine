#include "core/render/pbr/pbr_functions.wgsl"

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>
};

// Camera uniform buffer
struct Camera {
    world_to_view: mat4x4<f32>,
    view_to_ndc: mat4x4<f32>,
    ndc_to_view: mat4x4<f32>,
    view_to_world: mat4x4<f32>,
    position: vec4<f32>
}
@group(0) @binding(0) var<uniform> in_camera: Camera;

// G-Buffer textures
// Depth texture: Linear view-space depth, stored in R32Float
@group(1) @binding(0) var in_depth_texture: texture_2d<f32>;
@group(1) @binding(1) var in_depth_sampler: sampler;
@group(1) @binding(2) var in_albedo_metallic_t:  texture_2d<f32>;
@group(1) @binding(3) var in_albedo_metallic_s:  sampler;
@group(1) @binding(4) var in_normal_roughness_t: texture_2d<f32>;
@group(1) @binding(5) var in_normal_roughness_s: sampler;

// Light storage buffer (Light type comes from pbr_functions.wgsl)
@group(2) @binding(0) var<storage, read> in_lights: array<Light>;



/// Reconstruct world position from screen UV and linear view-space depth.
fn world_from_screen_coord_depth(uv: vec2<f32>, view_z: f32) -> vec3<f32> {
    let ndc_x = uv.x * 2.0 - 1.0;
    let ndc_y = (1.0 - uv.y) * 2.0 - 1.0; // Flip Y for NDC
    let view_pos4 = in_camera.ndc_to_view * vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    let view_dir = normalize(view_pos4.xyz / view_pos4.w);
    let view_position = view_dir * (view_z / -view_dir.z); // scale so .z == -view_z
    let world_pos4 = in_camera.view_to_world * vec4<f32>(view_position, 1.0);
    return world_pos4.xyz / world_pos4.w;
}



@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Reconstruct world position from linear view-space depth
    let view_z = textureSample(in_depth_texture, in_depth_sampler, in.tex_coord).r;
    let world_position = world_from_screen_coord_depth(in.tex_coord, view_z);

    // Discard background pixels (no geometry written to G-buffer)
    if view_z <= 0.0 {
        discard;
    }

    // Read G-Buffer
    let tmp_g_albedo_metallic  = textureSample(in_albedo_metallic_t, in_albedo_metallic_s, in.tex_coord);
    let tmp_g_normal_roughness = textureSample(in_normal_roughness_t, in_normal_roughness_s, in.tex_coord);
    let albedo    = tmp_g_albedo_metallic.rgb;
    let metallic  = tmp_g_albedo_metallic.a;
    let normal    = normalize(tmp_g_normal_roughness.xyz);
    let roughness = tmp_g_normal_roughness.a;

    // View direction from camera to fragment
    let view_dir = normalize(in_camera.position.xyz - world_position);

    // Base reflectivity F0: dielectrics ≈ 0.04, metals use albedo
    var f0 = vec3<f32>(0.04);
    f0 = mix(f0, albedo, metallic);

    // Accumulate lighting from each light source
    var lo = vec3<f32>(0.0);
    let lights_count = i32(arrayLength(&in_lights));
    for (var i = 0; i < lights_count; i = i + 1) {
        let light_data = get_light_data(in_lights[i], world_position);
        let n_dot_l = max(dot(normal, light_data.light_dir), 0.0);
        lo += brdf_for_light(normal, view_dir, light_data.light_dir, albedo, metallic, roughness, f0)
            * light_data.radiance
            * n_dot_l;
    }

    // Ambient term (placeholder; replace with IBL cubemap for full PBR)
    let n_dot_v = max(dot(normal, view_dir), 0.0);
    let ks = fresnel_schlick(n_dot_v, f0);
    let kd = (1.0 - ks) * (1.0 - metallic);
    let irradiance = vec3<f32>(1.0) * 0.001;
    let ambient = kd * irradiance * albedo;

    // HDR tone mapping (Reinhard) and final output
    var color = lo + ambient;
    color = color / (color + vec3<f32>(1.0));
    return vec4<f32>(color, 1.0);
}
