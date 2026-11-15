// Separable depth-aware blur of the half resolution ambient occlusion (r = visibility, g = linear depth).
#include "core/render/pbr/ambient_occlusion.wgsl"

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>
};

struct BlurPass {
    axis: u32 // 0 = horizontal, 1 = vertical
};
var<push_constant> in_pass: BlurPass;

@group(0) @binding(0) var in_source: texture_2d<f32>;
@group(0) @binding(1) var<uniform> in_ao: AmbientOcclusionParams;

const BLUR_RADIUS: i32 = 4;
const BLUR_SIGMA: f32 = 2.5;

fn load(pixel: vec2<i32>, size: vec2<i32>) -> vec2<f32> {
    return textureLoad(in_source, clamp(pixel, vec2<i32>(0), size - 1), 0).rg;
}

@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(in.clip_position.xy);
    let size = vec2<i32>(textureDimensions(in_source));
    let center = load(pixel, size);
    if center.y <= 0.0 {
        return vec4<f32>(1.0, 0.0, 0.0, 0.0);
    }
    let stride = select(vec2<i32>(0, 1), vec2<i32>(1, 0), in_pass.axis == 0u);

    // Depth slope along the axis, from the flattest side, so that tilted surfaces are not taken for edges
    let before = center.y - load(pixel - stride, size).y;
    let after = load(pixel + stride, size).y - center.y;
    let slope = select(before, after, abs(after) < abs(before));

    let sharpness = in_ao.quality.w / center.y;
    var sum = center.x;
    var weight_sum = 1.0;
    for (var i = -BLUR_RADIUS; i <= BLUR_RADIUS; i++) {
        if i == 0 {
            continue;
        }
        let tap = load(pixel + stride * i, size);
        let x = f32(i);
        let expected = center.y + slope * x;
        let w = select(0.0, exp(-x * x / (2.0 * BLUR_SIGMA * BLUR_SIGMA) - abs(tap.y - expected) * sharpness), tap.y > 0.0);
        sum += tap.x * w;
        weight_sum += w;
    }
    return vec4<f32>(sum / weight_sum, center.y, 0.0, 0.0);
}
