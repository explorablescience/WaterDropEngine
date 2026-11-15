#include "core/render/post_process/post_process_params.wgsl"

@group(0) @binding(0) var in_source_t: texture_2d<f32>;
@group(0) @binding(1) var in_source_s: sampler;
@group(1) @binding(0) var<uniform> in_params: PostProcessParams;

fn tap(uv: vec2<f32>, texel: vec2<f32>, x: f32, y: f32) -> vec3<f32> {
    return textureSampleLevel(in_source_t, in_source_s, uv + vec2<f32>(x, y) * texel, 0.0).rgb;
}

// 3x3 tent upsample, blended over the target by the pipeline
@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.tex_coord;
    let texel = 1.0 / vec2<f32>(textureDimensions(in_source_t));
    let color = tap(uv, texel, 0.0, 0.0) * 4.0
        + (tap(uv, texel, 0.0, 1.0) + tap(uv, texel, -1.0, 0.0) + tap(uv, texel, 1.0, 0.0) + tap(uv, texel, 0.0, -1.0)) * 2.0
        + (tap(uv, texel, -1.0, 1.0) + tap(uv, texel, 1.0, 1.0) + tap(uv, texel, -1.0, -1.0) + tap(uv, texel, 1.0, -1.0));
    return vec4<f32>(color / 16.0, 1.0);
}
