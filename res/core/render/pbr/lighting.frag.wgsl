#include "core/render/pbr/pbr_functions.wgsl"
#include "core/render/atmosphere/sky.wgsl"

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>
};

// Camera uniform buffer
@group(0) @binding(0) var<uniform> in_camera: Camera;

// G-Buffer textures, read at the pixel
// Depth texture: Linear view-space depth (-view_z), 0 where there is no geometry
@group(1) @binding(0) var in_depth_texture: texture_2d<f32>;
@group(1) @binding(1) var in_albedo_metallic_t: texture_2d<f32>;
@group(1) @binding(2) var in_normal_roughness_t: texture_2d<f32>;
@group(1) @binding(3) var in_ao_t: texture_2d<f32>;

// Light storage buffer (Light type comes from pbr_functions.wgsl)
@group(2) @binding(0) var<storage, read> in_lights: array<Light>;
@group(2) @binding(1) var<uniform> in_atmosphere: AtmosphereParams;
@group(2) @binding(2) var<uniform> in_pbr_params: PbrParams;

// Shadow maps of the directional light (3 cascades), comparison sampler, and parameters
struct ShadowCascades {
    splits: vec4<f32>, // xyz = view depth where each cascade ends, w = blend fraction
    params: vec4<f32>  // x = normal bias in texels
}
@group(3) @binding(0) var in_shadow_map_0: texture_depth_2d;
@group(3) @binding(1) var in_shadow_map_1: texture_depth_2d;
@group(3) @binding(2) var in_shadow_map_2: texture_depth_2d;
@group(3) @binding(3) var in_shadow_sampler: sampler_comparison;
@group(3) @binding(4) var<uniform> in_shadow_params: array<ShadowParams, 3>;
@group(3) @binding(5) var<uniform> in_shadow_cascades: ShadowCascades;

// Steepest receiver slope (depth per lateral distance) accounted for by the depth bias
const SHADOW_MAX_SLOPE: f32 = 8.0;
// Constant depth bias, in shadow map texels
const SHADOW_CONSTANT_BIAS: f32 = 0.2;

// Bilinear comparison of the shadow map of a cascade: 1 when lit, 0 when in shadow
fn shadow_compare(idx: u32, uv: vec2<f32>, depth: f32) -> f32 {
    switch idx {
        case 0u: {
            return textureSampleCompareLevel(in_shadow_map_0, in_shadow_sampler, uv, depth);
        }
        case 1u: {
            return textureSampleCompareLevel(in_shadow_map_1, in_shadow_sampler, uv, depth);
        }
        default: {
            return textureSampleCompareLevel(in_shadow_map_2, in_shadow_sampler, uv, depth);
        }
    }
}

// Light reaching a point from a cascade, with PCF: 1 when lit, 0 when in shadow
fn cascade_visibility(idx: u32, world_pos: vec3<f32>, geo_normal: vec3<f32>) -> f32 {
    let params = in_shadow_params[idx];
    let m = params.view_proj;
    let texel_world = params.light_dir.w;
    let map_size = f32(textureDimensions(in_shadow_map_0).x);
    let texel_uv = 1.0 / map_size;

    // Normal offset against the acne
    let pos = world_pos + geo_normal * texel_world * in_shadow_cascades.params.x;
    let ndc = (m * vec4<f32>(pos, 1.0)).xyz;
    let uv = vec2<f32>(ndc.x * 0.5 + 0.5, -ndc.y * 0.5 + 0.5);
    if any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0)) || ndc.z > 1.0 {
        return 1.0;
    }

    // Receiver plane depth bias: depth gradient of the surface in (uv, depth) space, so that each PCF tap is compared
    // with the depth of the surface under it instead of the depth at the center
    let helper = select(vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(1.0, 0.0, 0.0), abs(geo_normal.y) > 0.9);
    let t1 = normalize(cross(geo_normal, helper));
    let t2 = cross(geo_normal, t1);
    let to_uv = vec3<f32>(0.5, -0.5, 1.0);
    let plane = cross((m * vec4<f32>(t1, 0.0)).xyz * to_uv, (m * vec4<f32>(t2, 0.0)).xyz * to_uv);
    var gradient = vec2<f32>(0.0);
    if abs(plane.z) > 1e-12 {
        gradient = -plane.xy / plane.z;
    }
    let max_gradient = SHADOW_MAX_SLOPE * texel_world * map_size / params.depth_range;
    let gradient_length = length(gradient);
    if gradient_length > max_gradient {
        gradient *= max_gradient / gradient_length;
    }

    // The 4 texels of a bilinear comparison are up to a texel away from the tap
    let bias = (abs(gradient.x) + abs(gradient.y)) * texel_uv
        + SHADOW_CONSTANT_BIAS * texel_world / params.depth_range;
    let depth = ndc.z - bias;

    let half_kernel = i32(params.kernel_half_size);
    let spacing = texel_uv * params.kernel_scale;
    var lit = 0.0;
    for (var x = -half_kernel; x <= half_kernel; x = x + 1) {
        for (var y = -half_kernel; y <= half_kernel; y = y + 1) {
            let offset = vec2<f32>(f32(x), f32(y)) * spacing;
            lit += shadow_compare(idx, uv + offset, depth + dot(gradient, offset));
        }
    }
    let taps = f32((2 * half_kernel + 1) * (2 * half_kernel + 1));
    return mix(1.0, lit / taps, params.shadow_intensity);
}

// Light reaching a point from the directional light: 1 when lit, 0 when in shadow.
// Each cascade is cross-faded with the next one at its end, and the last one with no shadow.
fn sample_cascaded_shadow(world_pos: vec3<f32>, geo_normal: vec3<f32>) -> f32 {
    let view_depth = -(in_camera.world_to_view * vec4<f32>(world_pos, 1.0)).z;
    let splits = in_shadow_cascades.splits;
    var idx = 0u;
    var split = splits.x;
    if view_depth >= splits.z {
        return 1.0;
    } else if view_depth >= splits.y {
        idx = 2u;
        split = splits.z;
    } else if view_depth >= splits.x {
        idx = 1u;
        split = splits.y;
    }

    var visibility = cascade_visibility(idx, world_pos, geo_normal);
    let blend_start = split * (1.0 - splits.w);
    if view_depth > blend_start {
        var next = 1.0;
        if idx < 2u {
            next = cascade_visibility(idx + 1u, world_pos, geo_normal);
        }
        visibility = mix(visibility, next, smoothstep(blend_start, split, view_depth));
    }
    return visibility;
}

@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Reconstruct world position from linear view-space depth
    let pixel = vec2<i32>(in.clip_position.xy);
    let depth_z = textureLoad(in_depth_texture, pixel, 0).r;
    let world_position = get_depth_world_position(in.tex_coord, depth_z, in_camera);

    // Geometric normal (without normal mapping), from the screen derivatives (before the discard, in uniform control flow)
    let geo_normal_raw = cross(dpdy(world_position), dpdx(world_position));
    let depth_discontinuity = abs(dpdx(depth_z)) + abs(dpdy(depth_z)) > 0.05 * depth_z;

    // Discard background pixels (no geometry written to G-buffer)
    if depth_z <= 0.0 {
        discard;
    }

    // Read G-Buffer
    let tmp_g_albedo_metallic  = textureLoad(in_albedo_metallic_t, pixel, 0);
    let tmp_g_normal_roughness = textureLoad(in_normal_roughness_t, pixel, 0);
    let albedo    = tmp_g_albedo_metallic.rgb;
    let metallic  = clamp(tmp_g_albedo_metallic.a * in_pbr_params.material_params.x, 0.0, 1.0);
    let normal    = normalize(tmp_g_normal_roughness.xyz);
    let roughness = clamp(tmp_g_normal_roughness.a * in_pbr_params.material_params.y, 0.0, 1.0);
    let ao        = textureLoad(in_ao_t, pixel, 0).r;

    // View direction from camera to fragment
    let view_dir = normalize(in_camera.position.xyz - world_position);

    // Base reflectivity F0: dielectrics ≈ 0.04, metals use albedo
    var f0 = vec3<f32>(0.04);
    f0 = mix(f0, albedo, metallic);

    // Shadow factor from the directional sun light
    var geo_normal = normal;
    if !depth_discontinuity && length(geo_normal_raw) > 0.0 {
        geo_normal = normalize(geo_normal_raw);
        geo_normal *= sign(dot(geo_normal, view_dir) + 1e-6);
    }
    let shadow = sample_cascaded_shadow(world_position, geo_normal);

    // Accumulate lighting from each light source
    var lo = vec3<f32>(0.0);
    let lights_count = i32(arrayLength(&in_lights));
    for (var i = 0; i < lights_count; i = i + 1) {
        let light_type = i32(in_lights[i].color_type.w);
        if light_type < 0 {
            break; // End of the lights in use
        }
        if light_type > 0 && distance(in_lights[i].position_intensity.xyz, world_position) > in_lights[i].direction_range.w {
            continue; // Out of range
        }
        let light_data = get_light_data(in_lights[i], world_position);
        let n_dot_l = max(dot(normal, light_data.light_dir), 0.0);
        let light_contribution = brdf_for_light(normal, view_dir, light_data.light_dir, albedo, metallic, roughness, f0)
            * light_data.radiance
            * n_dot_l
            * in_pbr_params.lighting_params.x;
        // Only the directional light (type 0) is attenuated by the shadow map
        let attenuation = select(1.0, shadow, light_type == 0);
        lo += light_contribution * attenuation;
    }

    // Ambient term: sample the sky
    let kd = 0.2;
    let irradiance = sky(normal, in_atmosphere) * in_pbr_params.lighting_params.y;
    let ambient = kd * irradiance * albedo * ao;

    // Linear HDR output, tonemapped in post-processing
    var color = lo + ambient;
    color = color * in_pbr_params.lighting_params.z;
    return vec4<f32>(color, 1.0);
}
