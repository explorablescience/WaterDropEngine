// FXAA 3.11 (quality): finds the contrasted edges, searches their ends along their direction, then
// resamples the image across the edge, proportionally to the distance to the closest end.
#include "core/render/post_process/post_process_params.wgsl"

@group(0) @binding(0) var in_ldr_t: texture_2d<f32>;
@group(0) @binding(1) var in_ldr_s: sampler;
@group(0) @binding(2) var<uniform> in_params: PostProcessParams;

const EDGE_THRESHOLD_MIN: f32 = 0.0312;
const EDGE_THRESHOLD_MAX: f32 = 0.125;
const SUBPIXEL_QUALITY: f32 = 0.75;
const ITERATIONS: i32 = 12;

// Step length of each edge search iteration
fn search_step(i: i32) -> f32 {
    if i < 5 {
        return 1.0;
    } else if i == 5 {
        return 1.5;
    } else if i < 10 {
        return 2.0;
    } else if i == 10 {
        return 4.0;
    }
    return 8.0;
}

fn sample_color(uv: vec2<f32>) -> vec3<f32> {
    return textureSampleLevel(in_ldr_t, in_ldr_s, uv, 0.0).rgb;
}

// Perceptual luma of a linear color
fn luma(color: vec3<f32>) -> f32 {
    return sqrt(dot(color, vec3<f32>(0.299, 0.587, 0.114)));
}

fn luma_at(uv: vec2<f32>) -> f32 {
    return luma(sample_color(uv));
}

@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.tex_coord;
    let center = sample_color(uv);
    if in_params.anti_aliasing.x != 1.0 {
        return vec4<f32>(center, 1.0);
    }
    let texel = 1.0 / vec2<f32>(textureDimensions(in_ldr_t));

    // Skip the pixels without a contrasted neighborhood
    let luma_c = luma(center);
    let luma_n = luma_at(uv + vec2<f32>(0.0, -texel.y));
    let luma_s = luma_at(uv + vec2<f32>(0.0, texel.y));
    let luma_w = luma_at(uv + vec2<f32>(-texel.x, 0.0));
    let luma_e = luma_at(uv + vec2<f32>(texel.x, 0.0));
    let luma_min = min(luma_c, min(min(luma_n, luma_s), min(luma_w, luma_e)));
    let luma_max = max(luma_c, max(max(luma_n, luma_s), max(luma_w, luma_e)));
    let range = luma_max - luma_min;
    if range < max(EDGE_THRESHOLD_MIN, luma_max * EDGE_THRESHOLD_MAX) {
        return vec4<f32>(center, 1.0);
    }

    let luma_nw = luma_at(uv + vec2<f32>(-texel.x, -texel.y));
    let luma_ne = luma_at(uv + vec2<f32>(texel.x, -texel.y));
    let luma_sw = luma_at(uv + vec2<f32>(-texel.x, texel.y));
    let luma_se = luma_at(uv + vec2<f32>(texel.x, texel.y));
    let luma_ns = luma_n + luma_s;
    let luma_we = luma_w + luma_e;
    let luma_w_corners = luma_nw + luma_sw;
    let luma_e_corners = luma_ne + luma_se;
    let luma_n_corners = luma_nw + luma_ne;
    let luma_s_corners = luma_sw + luma_se;

    // Edge orientation (a horizontal edge varies along y)
    let edge_h = abs(-2.0 * luma_w + luma_w_corners) + abs(-2.0 * luma_c + luma_ns) * 2.0
        + abs(-2.0 * luma_e + luma_e_corners);
    let edge_v = abs(-2.0 * luma_n + luma_n_corners) + abs(-2.0 * luma_c + luma_we) * 2.0
        + abs(-2.0 * luma_s + luma_s_corners);
    let is_horizontal = edge_h >= edge_v;

    // Side of the pixel where the edge is
    let luma_neg = select(luma_w, luma_n, is_horizontal);
    let luma_pos = select(luma_e, luma_s, is_horizontal);
    let gradient_neg = luma_neg - luma_c;
    let gradient_pos = luma_pos - luma_c;
    let neg_steepest = abs(gradient_neg) >= abs(gradient_pos);
    let gradient_scaled = 0.25 * max(abs(gradient_neg), abs(gradient_pos));
    var step_length = select(texel.x, texel.y, is_horizontal);
    var luma_local_avg = 0.5 * (luma_pos + luma_c);
    if neg_steepest {
        step_length = -step_length;
        luma_local_avg = 0.5 * (luma_neg + luma_c);
    }

    // Search the ends of the edge, from the middle of the edge
    let across = select(vec2<f32>(step_length * 0.5, 0.0), vec2<f32>(0.0, step_length * 0.5), is_horizontal);
    let along = select(vec2<f32>(0.0, texel.y), vec2<f32>(texel.x, 0.0), is_horizontal);
    let edge_uv = uv + across;
    var uv1 = edge_uv - along;
    var uv2 = edge_uv + along;
    var luma_end1 = luma_at(uv1) - luma_local_avg;
    var luma_end2 = luma_at(uv2) - luma_local_avg;
    var reached1 = abs(luma_end1) >= gradient_scaled;
    var reached2 = abs(luma_end2) >= gradient_scaled;
    if !reached1 {
        uv1 -= along;
    }
    if !reached2 {
        uv2 += along;
    }
    for (var i = 2; i < ITERATIONS && !(reached1 && reached2); i = i + 1) {
        if !reached1 {
            luma_end1 = luma_at(uv1) - luma_local_avg;
            reached1 = abs(luma_end1) >= gradient_scaled;
            if !reached1 {
                uv1 -= along * search_step(i);
            }
        }
        if !reached2 {
            luma_end2 = luma_at(uv2) - luma_local_avg;
            reached2 = abs(luma_end2) >= gradient_scaled;
            if !reached2 {
                uv2 += along * search_step(i);
            }
        }
    }

    // Offset towards the edge, if the luma variation at the closest end is coherent with the center
    let distance1 = select(uv.y - uv1.y, uv.x - uv1.x, is_horizontal);
    let distance2 = select(uv2.y - uv.y, uv2.x - uv.x, is_horizontal);
    let closest_is_1 = distance1 < distance2;
    let edge_length = distance1 + distance2;
    let pixel_offset = 0.5 - min(distance1, distance2) / edge_length;
    let center_smaller = luma_c < luma_local_avg;
    let end_negative = select(luma_end2 < 0.0, luma_end1 < 0.0, closest_is_1);
    var offset = select(0.0, pixel_offset, end_negative != center_smaller);

    // Sub-pixel aliasing (thin lines, isolated pixels)
    let luma_avg = (2.0 * (luma_ns + luma_we) + luma_w_corners + luma_e_corners) / 12.0;
    let subpixel = clamp(abs(luma_avg - luma_c) / range, 0.0, 1.0);
    let subpixel_smooth = (-2.0 * subpixel + 3.0) * subpixel * subpixel;
    offset = max(offset, subpixel_smooth * subpixel_smooth * SUBPIXEL_QUALITY);

    let final_uv = uv + select(vec2<f32>(offset * step_length, 0.0), vec2<f32>(0.0, offset * step_length), is_horizontal);
    return vec4<f32>(sample_color(final_uv), 1.0);
}
