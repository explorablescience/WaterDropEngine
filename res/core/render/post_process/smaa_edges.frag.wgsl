// SMAA first pass: luma edges with the left (r) and top (g) neighbors.
#include "core/render/post_process/smaa_common.wgsl"

@group(0) @binding(0) var in_ldr_t: texture_2d<f32>;
@group(0) @binding(1) var in_ldr_s: sampler;
@group(0) @binding(2) var<uniform> in_params: PostProcessParams;

// The texture is sRGB, while the luma edges are detected on gamma-corrected colors
fn luma_at(uv: vec2<f32>) -> f32 {
    let linear = textureSampleLevel(in_ldr_t, in_ldr_s, uv, 0.0).rgb;
    let srgb = select(
        1.055 * pow(linear, vec3<f32>(1.0 / 2.4)) - 0.055,
        linear * 12.92,
        linear <= vec3<f32>(0.0031308)
    );
    return dot(srgb, vec3<f32>(0.2126, 0.7152, 0.0722));
}

@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    let rt = smaa_rt_metrics(textureDimensions(in_ldr_t));
    let uv = in.tex_coord;
    let offset_0 = rt.xyxy * vec4<f32>(-1.0, 0.0, 0.0, -1.0) + uv.xyxy;
    let offset_1 = rt.xyxy * vec4<f32>(1.0, 0.0, 0.0, 1.0) + uv.xyxy;
    let offset_2 = rt.xyxy * vec4<f32>(-2.0, 0.0, 0.0, -2.0) + uv.xyxy;
    let threshold = vec2<f32>(in_params.smaa.x);

    let l = luma_at(uv);
    let l_left = luma_at(offset_0.xy);
    let l_top = luma_at(offset_0.zw);
    var delta = vec4<f32>(abs(l - vec2<f32>(l_left, l_top)), 0.0, 0.0);
    var edges = step(threshold, delta.xy);
    if dot(edges, vec2<f32>(1.0)) == 0.0 {
        discard;
    }

    let l_right = luma_at(offset_1.xy);
    let l_bottom = luma_at(offset_1.zw);
    delta = vec4<f32>(delta.xy, abs(l - vec2<f32>(l_right, l_bottom)));
    var max_delta = max(delta.xy, delta.zw);

    let l_left_left = luma_at(offset_2.xy);
    let l_top_top = luma_at(offset_2.zw);
    delta = vec4<f32>(delta.xy, abs(vec2<f32>(l_left, l_top) - vec2<f32>(l_left_left, l_top_top)));
    max_delta = max(max_delta.xy, delta.zw);
    let final_delta = max(max_delta.x, max_delta.y);

    // Discards the edges next to a much more contrasted one
    edges *= step(vec2<f32>(final_delta), SMAA_LOCAL_CONTRAST_ADAPTATION_FACTOR * delta.xy);
    return vec4<f32>(edges, 0.0, 1.0);
}
