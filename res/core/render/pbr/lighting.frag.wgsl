#include "core/render/pbr/pbr_functions.wgsl"
#include "core/render/atmosphere/mars_sky.wgsl"

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>
};

// Camera uniform buffer
@group(0) @binding(0) var<uniform> in_camera: Camera;

// G-Buffer textures
// Depth texture: Linear view-space depth (-view_z), stored in R16Float
@group(1) @binding(0) var in_depth_texture: texture_2d<f32>;
@group(1) @binding(1) var in_depth_sampler: sampler;
@group(1) @binding(2) var in_albedo_metallic_t:  texture_2d<f32>;
@group(1) @binding(3) var in_albedo_metallic_s:  sampler;
@group(1) @binding(4) var in_normal_roughness_t: texture_2d<f32>;
@group(1) @binding(5) var in_normal_roughness_s: sampler;
@group(1) @binding(6) var in_ao_t: texture_2d<f32>;
@group(1) @binding(7) var in_ao_s: sampler;

// Light storage buffer (Light type comes from pbr_functions.wgsl)
@group(2) @binding(0) var<storage, read> in_lights: array<Light>;
@group(2) @binding(1) var<uniform> in_atmosphere: AtmosphereParams;
@group(2) @binding(2) var<uniform> in_pbr_params: PbrParams;

// Shadow map: depth textures from the directional sun light (3 cascades), sampler, and light matrices
@group(3) @binding(0) var in_shadow_map_0: texture_depth_2d;
@group(3) @binding(1) var in_shadow_map_1: texture_depth_2d;
@group(3) @binding(2) var in_shadow_map_2: texture_depth_2d;
@group(3) @binding(3) var in_shadow_sampler: sampler;
@group(3) @binding(4) var<uniform> in_shadow_params: array<ShadowParams, 3>;
@group(3) @binding(5) var<uniform> in_cascade_splits: vec4<f32>; // xyz = split distances, w = unused


fn map(value: f32, in_min: f32, in_max: f32, out_min: f32, out_max: f32) -> f32 {
    return (value - in_min) / (in_max - in_min) * (out_max - out_min) + out_min;
}

fn depth_pcf_sample(idx: u32, uv: vec2<f32>, depth: f32, shadow_map: texture_depth_2d) -> f32 {
    // Get PCF kernel parameters
    let shadow_params = in_shadow_params[idx];
    let half_kernel = i32(shadow_params.kernel_half_size);
    let kernel_scale = shadow_params.kernel_scale;
    let shadow_intensity = shadow_params.shadow_intensity;

    // PCF sampling
    var shadow = 0.0;
    let texel_size = 1.0 / vec2<f32>(textureDimensions(shadow_map, 0));
    for (var x = -half_kernel; x < half_kernel; x = x + 1) {
        for (var y = -half_kernel; y < half_kernel; y = y + 1) {
            let offset = vec2<f32>(f32(x), f32(y)) * texel_size * kernel_scale;
            let sample_depth = textureSample(shadow_map, in_shadow_sampler, uv + offset);
            if (depth > sample_depth) {
                shadow += 1.0;
            }
        }
    }
    return shadow / f32(4 * half_kernel * half_kernel) * shadow_intensity;
}

// Returns 1.0 when the fragment is lit, 0.0 when in shadow. Uses 3x3 PCF for smooth shadows.
// Selects appropriate cascade based on linear depth from camera.
fn sample_cascaded_shadow(world_pos: vec3<f32>) -> f32 {
    // Compute linear depth from camera
    let linear_depth = distance(in_camera.position.xyz, world_pos);

    // Select cascade based on split distances
    var cascade_idx = 0u;
    if linear_depth < in_cascade_splits.x {
        cascade_idx = 0u;
    } else if linear_depth < in_cascade_splits.y {
        cascade_idx = 1u;
    } else {
        cascade_idx = 2u;
    }

    let shadow_params = in_shadow_params[cascade_idx];

    // Project into light space
    let light_clip = shadow_params.view_proj * vec4<f32>(world_pos, 1.0);
    let ndc = light_clip.xyz / light_clip.w;

    // Discard fragments outside the light frustum (treat as fully lit)
    if ndc.x < -1.0 || ndc.x > 1.0 || ndc.y < -1.0 || ndc.y > 1.0 || ndc.z < 0.0 || ndc.z > 1.0 {
        return 1.0;
    }

    // NDC Y=1 is top; UV Y=0 is top — flip Y
    let shadow_uv = vec2<f32>(ndc.x * 0.5 + 0.5, -ndc.y * 0.5 + 0.5);
    let bias = shadow_params.light_dir.w * 0.3;

    // PCF sampling for smooth shadow edges
    var shadow = 0.0;
    if cascade_idx == 0u { // Wgsl doesn't allow dynamic indexing of arrays of textures, so we need to branch here
        shadow = depth_pcf_sample(cascade_idx, shadow_uv, ndc.z - bias, in_shadow_map_0);
    } else if cascade_idx == 1u {
        shadow = depth_pcf_sample(cascade_idx, shadow_uv, ndc.z - bias, in_shadow_map_1);
    } else {
        shadow = depth_pcf_sample(cascade_idx, shadow_uv, ndc.z - bias, in_shadow_map_2);
    }
    return 1.0 - shadow; // Invert: 1.0 = fully lit, 0.0 = fully shadowed
}

@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Reconstruct world position from linear view-space depth
    let depth_z = textureSample(in_depth_texture, in_depth_sampler, in.tex_coord).r;
    let world_position = get_depth_world_position(in.tex_coord, depth_z, in_camera);

    // Discard background pixels (no geometry written to G-buffer)
    if depth_z <= 0.0 {
        discard;
    }

    // Read G-Buffer
    let tmp_g_albedo_metallic  = textureSample(in_albedo_metallic_t, in_albedo_metallic_s, in.tex_coord);
    let tmp_g_normal_roughness = textureSample(in_normal_roughness_t, in_normal_roughness_s, in.tex_coord);
    let albedo    = tmp_g_albedo_metallic.rgb;
    let metallic  = clamp(tmp_g_albedo_metallic.a * in_pbr_params.material_params.x, 0.0, 1.0);
    let normal    = normalize(tmp_g_normal_roughness.xyz);
    let roughness = clamp(tmp_g_normal_roughness.a * in_pbr_params.material_params.y, 0.0, 1.0);
    let ao        = textureSample(in_ao_t, in_ao_s, in.tex_coord).r;

    // View direction from camera to fragment
    let view_dir = normalize(in_camera.position.xyz - world_position);

    // Base reflectivity F0: dielectrics ≈ 0.04, metals use albedo
    var f0 = vec3<f32>(0.04);
    f0 = mix(f0, albedo, metallic);

    // Shadow factor from the directional sun light
    let shadow = sample_cascaded_shadow(world_position);

    // Accumulate lighting from each light source
    var lo = vec3<f32>(0.0);
    let lights_count = i32(arrayLength(&in_lights));
    for (var i = 0; i < lights_count; i = i + 1) {
        let light_data = get_light_data(in_lights[i], world_position);
        let n_dot_l = max(dot(normal, light_data.light_dir), 0.0);
        let light_contribution = brdf_for_light(normal, view_dir, light_data.light_dir, albedo, metallic, roughness, f0)
            * light_data.radiance
            * n_dot_l
            * in_pbr_params.lighting_params.x;
        // Only the directional light (type 0) is attenuated by the shadow map
        let light_type = i32(in_lights[i].color_type.w);
        let attenuation = select(1.0, shadow, light_type == 0);
        lo += light_contribution * attenuation;
    }

    // Ambient term: sample the Mars sky
    let kd = 0.2;
    let irradiance = mars_sky(normal, in_atmosphere) * in_pbr_params.lighting_params.y;
    let ambient = kd * irradiance * albedo * ao;

    // HDR tone mapping (Reinhard) and final output
    var color = lo + ambient;
    color = color * in_pbr_params.lighting_params.z;
    color = color / (color + vec3<f32>(in_pbr_params.lighting_params.w));
    return vec4<f32>(color, 1.0);
}
