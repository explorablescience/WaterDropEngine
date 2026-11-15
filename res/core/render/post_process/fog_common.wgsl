// ================================================================
// Exponential height fog, blended over the scene with its alpha.
// Included by fog.frag.wgsl and fog_sky.frag.wgsl, which define
// `fn fog_base_color(ray: vec3<f32>) -> vec3<f32>`.
// ================================================================
#include "core/render/pbr/pbr_functions.wgsl"

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>
};

struct FogParams {
    color: vec4<f32>,      // xyz = color, w = sky color amount
    params: vec4<f32>,     // x = density, y = height falloff, z = base height, w = start distance
    limits: vec4<f32>,     // x = max opacity, y = sky distance, z = enabled
    sun: vec4<f32>,        // xyz = sun color, w = sun exponent
    sun_params: vec4<f32>  // x = sun intensity
};

@group(0) @binding(0) var<uniform> in_camera: Camera;

// Resolved linear view-space depth, 0 where there is no geometry
@group(1) @binding(0) var in_depth_t: texture_2d<f32>;
@group(1) @binding(1) var in_depth_s: sampler;
@group(1) @binding(2) var<uniform> in_fog: FogParams;

@group(2) @binding(0) var<storage, read> in_lights: array<Light>;

fn view_ray(uv: vec2<f32>) -> vec3<f32> {
    let ndc = vec2<f32>(uv.x * 2.0 - 1.0, (1.0 - uv.y) * 2.0 - 1.0);
    let clip = in_camera.ndc_to_view * vec4<f32>(ndc, 1.0, 1.0);
    let view_dir = normalize(clip.xyz / clip.w);
    return normalize((in_camera.view_to_world * vec4<f32>(view_dir, 0.0)).xyz);
}

// Integral of the density `density * exp(-falloff * (height - base_height))` along the ray
fn fog_optical_depth(origin: vec3<f32>, ray: vec3<f32>, distance: f32) -> f32 {
    let falloff = in_fog.params.y;
    let density = in_fog.params.x * exp(clamp(-falloff * (origin.y - in_fog.params.z), -80.0, 80.0));
    let k = falloff * ray.y * distance;
    if abs(k) < 1e-4 {
        return density * distance;
    }
    return density * distance * (1.0 - exp(min(-k, 80.0))) / k;
}

// Light of the first directional light scattered towards the camera
fn fog_sun_scattering(ray: vec3<f32>) -> vec3<f32> {
    if in_fog.sun_params.x <= 0.0 {
        return vec3<f32>(0.0);
    }
    let lights_count = arrayLength(&in_lights);
    for (var i = 0u; i < lights_count; i = i + 1u) {
        let light_type = i32(in_lights[i].color_type.w);
        if light_type < 0 {
            break;
        }
        if light_type == 0 {
            let to_light = normalize(-in_lights[i].direction_range.xyz);
            let lobe = pow(max(dot(ray, to_light), 0.0), in_fog.sun.w);
            let light_intensity = clamp(in_lights[i].position_intensity.w, 0.0, 1.0);
            return in_fog.sun.rgb * lobe * in_fog.sun_params.x * light_intensity;
        }
    }
    return vec3<f32>(0.0);
}

@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    let depth = textureSample(in_depth_t, in_depth_s, in.tex_coord).r;
    let camera_pos = in_camera.position.xyz;

    var ray = view_ray(in.tex_coord);
    var distance = in_fog.limits.y;
    if depth > 0.0 {
        let to_frag = get_depth_world_position(in.tex_coord, depth, in_camera) - camera_pos;
        distance = length(to_frag);
        ray = to_frag / max(distance, 1e-4);
    }

    let start = min(in_fog.params.w, distance);
    let optical_depth = fog_optical_depth(camera_pos + ray * start, ray, distance - start);
    let amount = min(1.0 - exp(-optical_depth), in_fog.limits.x);
    if amount <= 0.0 {
        discard;
    }

    let color = fog_base_color(ray) + fog_sun_scattering(ray);
    return vec4<f32>(color, amount);
}
