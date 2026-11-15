// SMAA third pass: blends each pixel with its neighbors by the weights of the second pass.
#include "core/render/post_process/smaa_common.wgsl"

@group(0) @binding(0) var in_ldr_t: texture_2d<f32>;
@group(0) @binding(1) var in_ldr_s: sampler;
@group(0) @binding(2) var in_weights_t: texture_2d<f32>;
@group(0) @binding(3) var in_weights_s: sampler;
@group(0) @binding(4) var<uniform> in_params: PostProcessParams;

fn color_at(uv: vec2<f32>) -> vec4<f32> {
    return textureSampleLevel(in_ldr_t, in_ldr_s, uv, 0.0);
}

@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    let rt = smaa_rt_metrics(textureDimensions(in_ldr_t));
    let uv = in.tex_coord;
    let offset = rt.xyxy * vec4<f32>(1.0, 0.0, 0.0, 1.0) + uv.xyxy;

    // Right, top, bottom and left weights
    let a = vec4<f32>(
        textureSampleLevel(in_weights_t, in_weights_s, offset.xy, 0.0).a,
        textureSampleLevel(in_weights_t, in_weights_s, offset.zw, 0.0).g,
        textureSampleLevel(in_weights_t, in_weights_s, uv, 0.0).zx
    );
    if dot(a, vec4<f32>(1.0)) < 1e-5 {
        return vec4<f32>(color_at(uv).rgb, 1.0);
    }

    // Blends along the strongest direction, with the bilinear filtering mixing the pixel and its neighbor
    let horizontal = max(a.x, a.z) > max(a.y, a.w);
    let blending_offset = select(vec4<f32>(0.0, a.y, 0.0, a.w), vec4<f32>(a.x, 0.0, a.z, 0.0), horizontal);
    var blending_weight = select(a.yw, a.xz, horizontal);
    blending_weight /= dot(blending_weight, vec2<f32>(1.0));
    let blending_coord = blending_offset * vec4<f32>(rt.xy, -rt.xy) + uv.xyxy;
    let color = blending_weight.x * color_at(blending_coord.xy) + blending_weight.y * color_at(blending_coord.zw);
    return vec4<f32>(color.rgb, 1.0);
}
