#include "core/render/post_process/post_process_params.wgsl"

// The first downsample (from the render texture) applies the threshold and a Karis average against fireflies
struct BloomPass {
    first: u32
};
var<push_constant> in_pass: BloomPass;

@group(0) @binding(0) var in_source_t: texture_2d<f32>;
@group(0) @binding(1) var in_source_s: sampler;
@group(1) @binding(0) var<uniform> in_params: PostProcessParams;

fn tap(uv: vec2<f32>, texel: vec2<f32>, x: f32, y: f32) -> vec3<f32> {
    return textureSampleLevel(in_source_t, in_source_s, uv + vec2<f32>(x, y) * texel, 0.0).rgb;
}

fn soft_threshold(color: vec3<f32>) -> vec3<f32> {
    let threshold = in_params.bloom.x;
    let knee = in_params.bloom.y;
    let brightness = max(color.r, max(color.g, color.b));
    var soft = clamp(brightness - threshold + knee, 0.0, 2.0 * knee);
    soft = soft * soft / (4.0 * knee + 1e-5);
    return color * max(soft, brightness - threshold) / max(brightness, 1e-5);
}

fn karis_weight(color: vec3<f32>) -> f32 {
    return 1.0 / (1.0 + luminance(color));
}

// 13-tap downsample (Jimenez 2014)
@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.tex_coord;
    let texel = 1.0 / vec2<f32>(textureDimensions(in_source_t));
    let a = tap(uv, texel, -2.0, 2.0);
    let b = tap(uv, texel, 0.0, 2.0);
    let c = tap(uv, texel, 2.0, 2.0);
    let d = tap(uv, texel, -2.0, 0.0);
    let e = tap(uv, texel, 0.0, 0.0);
    let f = tap(uv, texel, 2.0, 0.0);
    let g = tap(uv, texel, -2.0, -2.0);
    let h = tap(uv, texel, 0.0, -2.0);
    let i = tap(uv, texel, 2.0, -2.0);
    let j = tap(uv, texel, -1.0, 1.0);
    let k = tap(uv, texel, 1.0, 1.0);
    let l = tap(uv, texel, -1.0, -1.0);
    let m = tap(uv, texel, 1.0, -1.0);

    var color: vec3<f32>;
    if in_pass.first == 1u {
        let g0 = (a + b + d + e) * 0.25;
        let g1 = (b + c + e + f) * 0.25;
        let g2 = (d + e + g + h) * 0.25;
        let g3 = (e + f + h + i) * 0.25;
        let g4 = (j + k + l + m) * 0.25;
        let w0 = 0.125 * karis_weight(g0);
        let w1 = 0.125 * karis_weight(g1);
        let w2 = 0.125 * karis_weight(g2);
        let w3 = 0.125 * karis_weight(g3);
        let w4 = 0.5 * karis_weight(g4);
        color = (g0 * w0 + g1 * w1 + g2 * w2 + g3 * w3 + g4 * w4) / (w0 + w1 + w2 + w3 + w4);
        color = soft_threshold(min(max(color, vec3<f32>(0.0)), vec3<f32>(65000.0)));
    } else {
        color = e * 0.125
            + (a + c + g + i) * 0.03125
            + (b + d + f + h) * 0.0625
            + (j + k + l + m) * 0.125;
    }
    return vec4<f32>(color, 1.0);
}
