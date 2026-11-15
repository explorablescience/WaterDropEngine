// Temporal anti-aliasing: blends the frame with the history reprojected by the camera motion, clipped to the colors
// around the pixel. Colors are blended in a reversibly tonemapped YCoCg space, so that bright pixels do not dominate.
#include "core/render/post_process/post_process_params.wgsl"

struct Camera {
    world_to_view: mat4x4<f32>,
    view_to_ndc: mat4x4<f32>,
    ndc_to_view: mat4x4<f32>,
    view_to_world: mat4x4<f32>,
    position: vec4<f32>,
    world_to_ndc_unjittered: mat4x4<f32>,
    prev_world_to_ndc: mat4x4<f32>,
    jitter: vec4<f32>
}
@group(0) @binding(0) var<uniform> in_camera: Camera;

@group(1) @binding(0) var in_color_t: texture_2d<f32>;
// Linear view-space depth, 0 where there is no geometry
@group(1) @binding(1) var in_depth_t: texture_2d<f32>;
// Alpha is 0 until the history holds a frame
@group(1) @binding(2) var in_history_t: texture_2d<f32>;
@group(1) @binding(3) var in_history_s: sampler;
@group(1) @binding(4) var<uniform> in_params: PostProcessParams;

// Half-size of the color box the history is clipped to, in standard deviations of the neighborhood
const VARIANCE_GAMMA: f32 = 1.25;
// How much the history is clipped where the camera does not move. Details thinner than a pixel are missing from
// the neighborhood in some frames, and a full clipping would make them blink; it still erases the trails of the
// objects moving on their own
const STATIC_CLIPPING: f32 = 0.1;
// Camera motion in pixels from which the history is fully clipped
const MOVING_PIXELS: f32 = 0.5;

fn max3(c: vec3<f32>) -> f32 {
    return max(c.r, max(c.g, c.b));
}

fn to_blend_space(rgb: vec3<f32>) -> vec3<f32> {
    let c = clamp(rgb, vec3<f32>(0.0), vec3<f32>(65504.0));
    let t = c / (1.0 + max3(c));
    return vec3<f32>(
        0.25 * t.r + 0.5 * t.g + 0.25 * t.b,
        0.5 * t.r - 0.5 * t.b,
        -0.25 * t.r + 0.5 * t.g - 0.25 * t.b
    );
}

fn from_blend_space(ycocg: vec3<f32>) -> vec3<f32> {
    let tmp = ycocg.x - ycocg.z;
    let t = max(vec3<f32>(tmp + ycocg.y, ycocg.x + ycocg.z, tmp - ycocg.y), vec3<f32>(0.0));
    return t / max(1.0 - max3(t), 1e-4);
}

// Homogeneous world position seen at `uv`: a point, or a direction (w = 0) for the sky
fn world_position(uv: vec2<f32>, depth: f32) -> vec4<f32> {
    let ndc = vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 0.0, 1.0);
    let view_ray = in_camera.ndc_to_view * ndc;
    let ray = view_ray.xyz / view_ray.w;
    if depth <= 0.0 {
        return vec4<f32>((in_camera.view_to_world * vec4<f32>(ray, 0.0)).xyz, 0.0);
    }
    return in_camera.view_to_world * vec4<f32>(ray * (-depth / ray.z), 1.0);
}

fn clip_to_uv(clip: vec4<f32>) -> vec2<f32> {
    let ndc = clip.xy / clip.w;
    return vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
}

// Catmull-Rom filtering in 5 bilinear taps, which keeps the history sharp when it is resampled
fn sample_history(uv: vec2<f32>, size: vec2<f32>) -> vec4<f32> {
    let position = uv * size;
    let center = floor(position - 0.5) + 0.5;
    let f = position - center;
    let w0 = f * (-0.5 + f * (1.0 - 0.5 * f));
    let w1 = 1.0 + f * f * (-2.5 + 1.5 * f);
    let w2 = f * (0.5 + f * (2.0 - 1.5 * f));
    let w3 = f * f * (-0.5 + 0.5 * f);
    let w12 = w1 + w2;
    let uv0 = (center - 1.0) / size;
    let uv3 = (center + 2.0) / size;
    let uv12 = (center + w2 / w12) / size;

    var color = vec4<f32>(0.0);
    var weight = 0.0;
    for (var i = 0; i < 5; i++) {
        var tap: vec2<f32>;
        var w: f32;
        switch i {
            case 0: { tap = vec2<f32>(uv12.x, uv0.y); w = w12.x * w0.y; }
            case 1: { tap = vec2<f32>(uv0.x, uv12.y); w = w0.x * w12.y; }
            case 2: { tap = uv12; w = w12.x * w12.y; }
            case 3: { tap = vec2<f32>(uv3.x, uv12.y); w = w3.x * w12.y; }
            default: { tap = vec2<f32>(uv12.x, uv3.y); w = w12.x * w3.y; }
        }
        color += textureSampleLevel(in_history_t, in_history_s, tap, 0.0) * w;
        weight += w;
    }
    return color / weight;
}

// Moves `color` towards the box center until it is inside the box
fn clip_to_box(color: vec3<f32>, center: vec3<f32>, extent: vec3<f32>) -> vec3<f32> {
    let offset = color - center;
    let units = abs(offset / max(extent, vec3<f32>(1e-5)));
    let max_unit = max3(units);
    if max_unit > 1.0 {
        return center + offset / max_unit;
    }
    return color;
}

@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    let size = vec2<i32>(textureDimensions(in_color_t));
    let pixel = vec2<i32>(in.clip_position.xy);
    let texel = 1.0 / vec2<f32>(size);
    let uv = (vec2<f32>(pixel) + 0.5) * texel;

    // Each sample shows the scene at its pixel center minus the jitter, in pixels (NDC y is up)
    let jitter = vec2<f32>(in_camera.jitter.x, -in_camera.jitter.y) * vec2<f32>(size) * 0.5;

    // Color distribution and closest surface of the 3x3 neighborhood, and the color at the pixel center
    // reconstructed from the samples by a Blackman-Harris fit, which is more stable than the center sample
    var current = vec3<f32>(0.0);
    var current_weight = 0.0;
    var m1 = vec3<f32>(0.0);
    var m2 = vec3<f32>(0.0);
    var closest_depth = 0.0;
    var closest_offset = vec2<i32>(0);
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            let p = clamp(pixel + vec2<i32>(x, y), vec2<i32>(0), size - 1);
            let color = to_blend_space(textureLoad(in_color_t, p, 0).rgb);
            m1 += color;
            m2 += color * color;
            let d = vec2<f32>(f32(x), f32(y)) - jitter;
            let w = exp(-2.29 * dot(d, d));
            current += color * w;
            current_weight += w;
            let depth = textureLoad(in_depth_t, p, 0).r;
            if depth > 0.0 && (closest_depth <= 0.0 || depth < closest_depth) {
                closest_depth = depth;
                closest_offset = vec2<i32>(x, y);
            }
        }
    }
    current /= current_weight;
    let mean = m1 / 9.0;
    let sigma = sqrt(max(m2 / 9.0 - mean * mean, vec3<f32>(0.0)));

    // Camera motion of the closest surface, so that the history of the foreground edges follows them
    let world = world_position(uv + vec2<f32>(closest_offset) * texel, closest_depth);
    let prev_clip = in_camera.prev_world_to_ndc * world;
    let motion = clip_to_uv(in_camera.world_to_ndc_unjittered * world) - clip_to_uv(prev_clip);
    let history_uv = uv - motion;

    let history = sample_history(history_uv, vec2<f32>(size));
    let off_screen = any(history_uv != clamp(history_uv, vec2<f32>(0.0), vec2<f32>(1.0)));
    if prev_clip.w <= 0.0 || off_screen || history.a < 0.5 {
        return vec4<f32>(from_blend_space(current), 1.0);
    }

    let history_color = to_blend_space(history.rgb);
    let clipped = clip_to_box(history_color, mean, sigma * VARIANCE_GAMMA);
    let clipping = mix(STATIC_CLIPPING, 1.0, saturate(length(motion * vec2<f32>(size)) / MOVING_PIXELS));
    let rectified = mix(history_color, clipped, clipping);
    return vec4<f32>(from_blend_space(mix(rectified, current, in_params.anti_aliasing.y)), 1.0);
}
