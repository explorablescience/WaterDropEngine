// Ground truth ambient occlusion (Jimenez et al. 2016, after XeGTAO), at half resolution.
// Outputs the visibility and the linear depth of the full resolution pixel it was computed at.
#include "core/render/pbr/pbr_functions.wgsl"
#include "core/render/pbr/ambient_occlusion.wgsl"

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>
};

@group(0) @binding(0) var<uniform> in_camera: Camera;
// Linear view-space depth (-view_z), 0 where there is no geometry
@group(1) @binding(0) var in_depth_texture: texture_2d<f32>;
@group(1) @binding(1) var<uniform> in_ao: AmbientOcclusionParams;

const HALF_PI: f32 = 1.5707963;
// Distance of the first sample in pixels, so that it leaves the center pixel
const MIN_SAMPLE_PIXELS: f32 = 1.3;

// View-space position of a full resolution pixel, for a symmetric perspective projection
fn view_position(pixel: vec2<i32>, depth: f32, size: vec2<f32>) -> vec3<f32> {
    let uv = (vec2<f32>(pixel) + 0.5) / size;
    let ndc = vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
    return vec3<f32>(ndc.x / in_camera.view_to_ndc[0][0], ndc.y / in_camera.view_to_ndc[1][1], -1.0) * depth;
}

fn load_view_position(pixel: vec2<i32>, size_i: vec2<i32>, size: vec2<f32>) -> vec3<f32> {
    let p = clamp(pixel, vec2<i32>(0), size_i - 1);
    return view_position(p, textureLoad(in_depth_texture, p, 0).r, size);
}

// Normal of the geometry from the depth, from the neighbors on the flattest side (normal maps would occlude flat surfaces)
fn geometric_normal(pixel: vec2<i32>, center: vec3<f32>, view_vec: vec3<f32>, size_i: vec2<i32>, size: vec2<f32>) -> vec3<f32> {
    let left = center - load_view_position(pixel - vec2<i32>(1, 0), size_i, size);
    let right = load_view_position(pixel + vec2<i32>(1, 0), size_i, size) - center;
    let up = center - load_view_position(pixel - vec2<i32>(0, 1), size_i, size);
    let down = load_view_position(pixel + vec2<i32>(0, 1), size_i, size) - center;
    let dx = select(left, right, abs(right.z) < abs(left.z));
    let dy = select(up, down, abs(down.z) < abs(up.z));
    let n = cross(dx, dy);
    if dot(n, n) < 1e-20 {
        return view_vec;
    }
    return normalize(n) * sign(dot(n, view_vec) + 1e-6);
}

// Cosine of the angle between the view vector and a sample, faded to the lowest horizon with the distance
fn horizon_cos(
    screen: vec2<f32>, center: vec3<f32>, view_vec: vec3<f32>, low: f32,
    falloff_mul: f32, falloff_add: f32, size_i: vec2<i32>, size: vec2<f32>
) -> f32 {
    let pixel = vec2<i32>(floor(screen));
    if any(pixel < vec2<i32>(0)) || any(pixel >= size_i) {
        return low;
    }
    let depth = textureLoad(in_depth_texture, pixel, 0).r;
    if depth <= 0.0 {
        return low;
    }
    let delta = view_position(pixel, depth, size) - center;
    let dist = max(length(delta), 1e-5);
    let weight = clamp(dist * falloff_mul + falloff_add, 0.0, 1.0);
    return mix(low, dot(delta / dist, view_vec), weight);
}

// 4×4 ordered dither, covered by the blur
fn bayer4(p: vec2<u32>) -> f32 {
    let a = (p.x ^ p.y) & 3u;
    let b = p.y & 3u;
    let i = ((a & 1u) << 3u) | ((b & 1u) << 2u) | (a & 2u) | ((b & 2u) >> 1u);
    return (f32(i) + 0.5) / 16.0;
}

fn interleaved_gradient_noise(p: vec2<f32>) -> f32 {
    return fract(52.9829189 * fract(dot(p, vec2<f32>(0.06711056, 0.00583715))));
}

@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    let half_pixel = vec2<i32>(in.clip_position.xy);
    let size_i = vec2<i32>(textureDimensions(in_depth_texture));
    let size = vec2<f32>(size_i);
    let pixel = min(half_pixel * 2, size_i - 1);
    let depth = textureLoad(in_depth_texture, pixel, 0).r;
    if depth <= 0.0 {
        return vec4<f32>(1.0, 0.0, 0.0, 0.0);
    }

    let center = view_position(pixel, depth, size);
    let view_vec = normalize(-center);
    let normal = geometric_normal(pixel, center, view_vec, size_i, size);

    let radius = in_ao.effect.x;
    let screen_radius = min(radius * in_camera.view_to_ndc[1][1] * 0.5 * size.y / depth, in_ao.quality.z);
    if screen_radius < MIN_SAMPLE_PIXELS {
        return vec4<f32>(1.0, depth, 0.0, 0.0);
    }

    // Weight of the samples: 1 up to the falloff, then down to 0 at the radius
    let falloff_range = radius * in_ao.effect.y;
    let falloff_mul = -1.0 / falloff_range;
    let falloff_add = (radius - falloff_range) / falloff_range + 1.0;

    let slice_count = u32(in_ao.quality.x);
    let step_count = u32(in_ao.quality.y);
    let slice_noise = bayer4(vec2<u32>(half_pixel));
    let step_noise = interleaved_gradient_noise(vec2<f32>(half_pixel));
    let min_s = MIN_SAMPLE_PIXELS / screen_radius;
    let center_screen = vec2<f32>(pixel) + 0.5;

    var visibility = 0.0;
    for (var slice = 0u; slice < slice_count; slice++) {
        let phi = (f32(slice) + slice_noise) * PI / f32(slice_count);
        let direction = vec3<f32>(cos(phi), sin(phi), 0.0);
        // The screen y goes down
        let omega = vec2<f32>(direction.x, -direction.y) * screen_radius;

        // Normal projected on the slice plane, and its angle to the view vector
        let ortho = direction - dot(direction, view_vec) * view_vec;
        let axis = normalize(cross(ortho, view_vec));
        let projected = normal - axis * dot(normal, axis);
        let projected_length = length(projected);
        let cos_n = clamp(dot(projected, view_vec) / max(projected_length, 1e-6), 0.0, 1.0);
        let n = sign(dot(ortho, projected)) * acos(cos_n);

        // Highest horizon on each side, starting from the tangent plane
        let low0 = cos(n + HALF_PI);
        let low1 = cos(n - HALF_PI);
        var horizon0 = low0;
        var horizon1 = low1;
        for (var j = 0u; j < step_count; j++) {
            var s = (f32(j) + step_noise) / f32(step_count);
            s = s * s + min_s;
            let offset = omega * s;
            horizon0 = max(horizon0, horizon_cos(center_screen + offset, center, view_vec, low0, falloff_mul, falloff_add, size_i, size));
            horizon1 = max(horizon1, horizon_cos(center_screen - offset, center, view_vec, low1, falloff_mul, falloff_add, size_i, size));
        }

        // Cosine-weighted visible arc between the two horizons
        let h0 = n + clamp(-acos(clamp(horizon1, -1.0, 1.0)) - n, -HALF_PI, HALF_PI);
        let h1 = n + clamp(acos(clamp(horizon0, -1.0, 1.0)) - n, -HALF_PI, HALF_PI);
        let sin_n = sin(n);
        let arc0 = (cos_n + 2.0 * h0 * sin_n - cos(2.0 * h0 - n)) * 0.25;
        let arc1 = (cos_n + 2.0 * h1 * sin_n - cos(2.0 * h1 - n)) * 0.25;
        visibility += projected_length * (arc0 + arc1);
    }
    visibility = clamp(visibility / f32(slice_count), 0.0, 1.0);
    return vec4<f32>(pow(visibility, in_ao.effect.z), depth, 0.0, 0.0);
}
