#include "core/render/pbr/pbr_functions.wgsl"
#include "core/render/pbr/environment.wgsl"
#include "core/render/pbr/ambient_occlusion.wgsl"

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
@group(1) @binding(4) var in_emissive_t: texture_2d<f32>;
// Half resolution screen-space ambient occlusion
@group(1) @binding(5) var in_occlusion_t: texture_2d<f32>;
@group(1) @binding(6) var<uniform> in_occlusion: AmbientOcclusionParams;

// Light storage buffer (Light type comes from pbr_functions.wgsl)
@group(2) @binding(0) var<storage, read> in_lights: array<Light>;
@group(2) @binding(2) var<uniform> in_pbr_params: PbrParams;
@group(2) @binding(3) var<uniform> in_environment_sh: EnvironmentSH;
@group(2) @binding(4) var in_environment_map: texture_cube<f32>;
@group(2) @binding(5) var in_environment_sampler: sampler;
@group(2) @binding(6) var in_environment_brdf: texture_2d<f32>;

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

// Cascade of a point: idx 3 beyond the last one, and blend = weight of the next one (or of no shadow after the
// last one) in the cross-fade at its end
struct CascadeSelection {
    idx: u32,
    blend: f32
}
fn select_cascade(world_pos: vec3<f32>) -> CascadeSelection {
    let view_depth = -(in_camera.world_to_view * vec4<f32>(world_pos, 1.0)).z;
    let splits = in_shadow_cascades.splits;
    var out = CascadeSelection(0u, 0.0);
    var split = splits.x;
    if view_depth >= splits.z {
        out.idx = 3u;
        return out;
    } else if view_depth >= splits.y {
        out.idx = 2u;
        split = splits.z;
    } else if view_depth >= splits.x {
        out.idx = 1u;
        split = splits.y;
    }
    let blend_start = split * (1.0 - splits.w);
    if view_depth > blend_start {
        out.blend = smoothstep(blend_start, split, view_depth);
    }
    return out;
}

// Light reaching a point from the directional light: 1 when lit, 0 when in shadow.
// Each cascade is cross-faded with the next one at its end, and the last one with no shadow.
fn sample_cascaded_shadow(world_pos: vec3<f32>, geo_normal: vec3<f32>) -> f32 {
    let cascade = select_cascade(world_pos);
    if cascade.idx > 2u {
        return 1.0;
    }
    var visibility = cascade_visibility(cascade.idx, world_pos, geo_normal);
    if cascade.blend > 0.0 {
        var next = 1.0;
        if cascade.idx < 2u {
            next = cascade_visibility(cascade.idx + 1u, world_pos, geo_normal);
        }
        visibility = mix(visibility, next, cascade.blend);
    }
    return visibility;
}

// Same as SHADOW_CASCADE_COLORS, then no cascade
const SHADOW_DEBUG_CASCADE_COLORS = array<vec3<f32>, 4>(
    vec3<f32>(1.0, 0.3, 0.3),
    vec3<f32>(0.3, 1.0, 0.3),
    vec3<f32>(0.35, 0.5, 1.0),
    vec3<f32>(1.0)
);

// Tint of the shadow cascades view: the color of the cascade, faded to the next one with stripes over the blend
fn shadow_debug_cascade_tint(world_pos: vec3<f32>, pixel: vec2<f32>) -> vec3<f32> {
    var colors = SHADOW_DEBUG_CASCADE_COLORS;
    let cascade = select_cascade(world_pos);
    let color = pow(colors[cascade.idx], vec3<f32>(2.2));
    if cascade.blend <= 0.0 {
        return color;
    }
    let next = pow(colors[cascade.idx + 1u], vec3<f32>(2.2));
    let stripe = fract((pixel.x + pixel.y) / 12.0) < 0.25;
    return mix(color, next, cascade.blend) * select(1.0, 0.35, stripe);
}

// Color of the shadow texels view: shadow map texels per screen pixel, checkered by texel when they are large on
// screen, darkened by the shadow. Takes the screen derivatives of the world position.
fn shadow_debug_texels(world_pos: vec3<f32>, world_dx: vec3<f32>, world_dy: vec3<f32>, shadow: f32) -> vec3<f32> {
    let cascade = select_cascade(world_pos);
    if cascade.idx > 2u {
        return vec3<f32>(0.2);
    }
    // Orthographic light projection: the derivatives map linearly to the shadow map
    let m = in_shadow_params[cascade.idx].view_proj;
    let map_size = f32(textureDimensions(in_shadow_map_0).x);
    let to_texels = 0.5 * map_size;
    let texels_per_pixel = max(
        length((m * vec4<f32>(world_dx, 0.0)).xy),
        length((m * vec4<f32>(world_dy, 0.0)).xy)
    ) * to_texels;
    var color = debug_ramp(log2(max(texels_per_pixel, 1e-6)) + 3.0);

    let texel = vec2<i32>(floor(((m * vec4<f32>(world_pos, 1.0)).xy + 1.0) * to_texels));
    let checker = f32((texel.x + texel.y) & 1);
    color *= 1.0 - 0.3 * checker * (1.0 - smoothstep(0.25, 0.5, texels_per_pixel));
    return color * (0.5 + 0.5 * shadow);
}

@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Reconstruct world position from linear view-space depth
    let pixel = vec2<i32>(in.clip_position.xy);
    let depth_z = textureLoad(in_depth_texture, pixel, 0).r;
    let world_position = get_depth_world_position(in.tex_coord, depth_z, in_camera);

    // Geometric normal (without normal mapping), from the screen derivatives (before the discard, in uniform control flow)
    let world_dx = dpdx(world_position);
    let world_dy = dpdy(world_position);
    let geo_normal_raw = cross(world_dy, world_dx);
    let depth_discontinuity = abs(dpdx(depth_z)) + abs(dpdy(depth_z)) > 0.05 * depth_z;

    // Discard background pixels (no geometry written to G-buffer)
    if depth_z <= 0.0 {
        discard;
    }

    // Read G-Buffer
    let tmp_g_albedo_metallic  = textureLoad(in_albedo_metallic_t, pixel, 0);
    let tmp_g_normal_roughness = textureLoad(in_normal_roughness_t, pixel, 0);
    var albedo    = tmp_g_albedo_metallic.rgb;
    var metallic  = clamp(tmp_g_albedo_metallic.a * in_pbr_params.material_params.x, 0.0, 1.0);
    let normal    = normalize(tmp_g_normal_roughness.xyz);
    let roughness = clamp(tmp_g_normal_roughness.a * in_pbr_params.material_params.y, 0.0, 1.0);
    let ao        = textureLoad(in_ao_t, pixel, 0).r;
    let emissive  = textureLoad(in_emissive_t, pixel, 0).rgb;
    var occlusion = 1.0;
    if in_occlusion.effect.w > 0.0 {
        occlusion = upsample_occlusion(in_occlusion_t, pixel, depth_z);
        if in_occlusion.debug.x > 0.0 {
            return vec4<f32>(vec3<f32>(occlusion), 1.0);
        }
    }

    let debug_view = pbr_debug_view(in_pbr_params);
    if debug_view == PBR_DEBUG_ALBEDO {
        return vec4<f32>(albedo, 1.0);
    } else if debug_view == PBR_DEBUG_NORMALS {
        return pbr_debug_data(normal * 0.5 + 0.5);
    } else if debug_view == PBR_DEBUG_ROUGHNESS {
        return pbr_debug_data(vec3<f32>(roughness));
    } else if debug_view == PBR_DEBUG_METALLIC {
        return pbr_debug_data(vec3<f32>(metallic));
    } else if debug_view == PBR_DEBUG_OCCLUSION {
        return pbr_debug_data(vec3<f32>(ao * occlusion));
    } else if debug_view == PBR_DEBUG_CLAY {
        albedo = vec3<f32>(PBR_DEBUG_CLAY_ALBEDO);
        metallic = 0.0;
    }

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
    if debug_view == PBR_DEBUG_SHADOW {
        return pbr_debug_data(vec3<f32>(shadow));
    } else if debug_view == PBR_DEBUG_SHADOW_TEXELS {
        return pbr_debug_data(shadow_debug_texels(world_position, world_dx, world_dy, shadow));
    }

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

    // Ambient light of the environment, occluded by the material and the screen-space occlusion
    let ambient = environment_lighting(
        in_environment_sh, in_environment_map, in_environment_sampler, in_environment_brdf,
        normal, view_dir, albedo, metallic, roughness, f0, ao * occlusion
    );
    let ambient_scale = in_pbr_params.lighting_params.y;

    // Linear HDR output, tonemapped in post-processing
    var color = pbr_debug_light(debug_view, lo, ambient.diffuse * ambient_scale, ambient.specular * ambient_scale, emissive);
    if debug_view == PBR_DEBUG_SHADOW_CASCADES {
        color *= shadow_debug_cascade_tint(world_position, in.clip_position.xy);
    }
    color = color * in_pbr_params.lighting_params.z;
    return vec4<f32>(color, 1.0);
}
