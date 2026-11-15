// SMAA second pass: searches the ends of the edges and their crossing edges, and looks up the area covered on
// each side of the edge. Outputs the blending weights with the left/right (rg) and top/bottom (ba) neighbors.
#include "core/render/post_process/smaa_common.wgsl"

@group(0) @binding(0) var in_edges_t: texture_2d<f32>;
@group(0) @binding(1) var in_edges_s: sampler;
@group(0) @binding(2) var in_area_t: texture_2d<f32>;
@group(0) @binding(3) var in_search_t: texture_2d<f32>;
@group(0) @binding(4) var<uniform> in_params: PostProcessParams;

var<private> rt: vec4<f32>;

fn edges_at(uv: vec2<f32>) -> vec2<f32> {
    return textureSampleLevel(in_edges_t, in_edges_s, uv, 0.0).rg;
}

// ── DIAGONALS ───────────────────────────────────────────────────

// Decodes two binary edges from a bilinear fetch between them
fn decode_diag_bilinear_access_2(in_e: vec2<f32>) -> vec2<f32> {
    var e = in_e;
    e.r = e.r * abs(5.0 * e.r - 5.0 * 0.75);
    return round(e);
}

fn decode_diag_bilinear_access_4(e: vec4<f32>) -> vec4<f32> {
    let e_rb = e.rb * abs(5.0 * e.rb - 5.0 * 0.75);
    return round(vec4<f32>(e_rb.x, e.g, e_rb.y, e.a));
}

fn search_diag_1(uv: vec2<f32>, dir: vec2<f32>, e: ptr<function, vec2<f32>>) -> vec2<f32> {
    var coord = vec4<f32>(uv, -1.0, 1.0);
    let t = vec3<f32>(rt.xy, 1.0);
    while coord.z < in_params.smaa.z - 1.0 && coord.w > 0.9 {
        coord = vec4<f32>(t * vec3<f32>(dir, 1.0) + coord.xyz, coord.w);
        *e = edges_at(coord.xy);
        coord.w = dot(*e, vec2<f32>(0.5));
    }
    return coord.zw;
}

fn search_diag_2(uv: vec2<f32>, dir: vec2<f32>, e: ptr<function, vec2<f32>>) -> vec2<f32> {
    var coord = vec4<f32>(uv, -1.0, 1.0);
    coord.x += 0.25 * rt.x;
    let t = vec3<f32>(rt.xy, 1.0);
    while coord.z < in_params.smaa.z - 1.0 && coord.w > 0.9 {
        coord = vec4<f32>(t * vec3<f32>(dir, 1.0) + coord.xyz, coord.w);
        *e = decode_diag_bilinear_access_2(edges_at(coord.xy));
        coord.w = dot(*e, vec2<f32>(0.5));
    }
    return coord.zw;
}

fn area_diag(dist: vec2<f32>, e: vec2<f32>, offset: f32) -> vec2<f32> {
    var uv = vec2<f32>(SMAA_AREATEX_MAX_DISTANCE_DIAG) * e + dist;
    uv = SMAA_AREATEX_PIXEL_SIZE * uv + 0.5 * SMAA_AREATEX_PIXEL_SIZE;
    // Diagonal areas are on the second half of the texture
    uv.x += 0.5;
    uv.y += SMAA_AREATEX_SUBTEX_SIZE * offset;
    return textureSampleLevel(in_area_t, in_edges_s, uv, 0.0).rg;
}

fn calculate_diag_weights(uv: vec2<f32>, e: vec2<f32>, subsample_indices: vec4<f32>) -> vec2<f32> {
    var weights = vec2<f32>(0.0);

    var d = vec4<f32>(0.0);
    var end = vec2<f32>(0.0);
    if e.r > 0.0 {
        let d_xz = search_diag_1(uv, vec2<f32>(-1.0, 1.0), &end);
        d = vec4<f32>(d_xz.x, d.y, d_xz.y, d.w);
        d.x += f32(end.y > 0.9);
    }
    let d_yw = search_diag_1(uv, vec2<f32>(1.0, -1.0), &end);
    d = vec4<f32>(d.x, d_yw.x, d.z, d_yw.y);

    if d.x + d.y > 2.0 {
        let coords = vec4<f32>(-d.x + 0.25, d.x, d.y, -d.y - 0.25) * rt.xyxy + uv.xyxy;
        var c = vec4<f32>(
            textureSampleLevel(in_edges_t, in_edges_s, coords.xy, 0.0, vec2<i32>(-1, 0)).rg,
            textureSampleLevel(in_edges_t, in_edges_s, coords.zw, 0.0, vec2<i32>(1, 0)).rg
        );
        c = decode_diag_bilinear_access_4(c.xyzw).yxwz;
        var cc = vec2<f32>(2.0) * c.xz + c.yw;
        // Removes the crossing edge if the end of the line was not found
        cc = select(cc, vec2<f32>(0.0), d.zw >= vec2<f32>(0.9));
        weights += area_diag(d.xy, cc, subsample_indices.z);
    }

    let d_xz = search_diag_2(uv, vec2<f32>(-1.0, -1.0), &end);
    if textureSampleLevel(in_edges_t, in_edges_s, uv, 0.0, vec2<i32>(1, 0)).r > 0.0 {
        let d_yw2 = search_diag_2(uv, vec2<f32>(1.0, 1.0), &end);
        d = vec4<f32>(d_xz.x, d_yw2.x, d_xz.y, d_yw2.y);
        d.y += f32(end.y > 0.9);
    } else {
        d = vec4<f32>(d_xz.x, 0.0, d_xz.y, 0.0);
    }

    if d.x + d.y > 2.0 {
        let coords = vec4<f32>(-d.x, -d.x, d.y, d.y) * rt.xyxy + uv.xyxy;
        let c = vec4<f32>(
            textureSampleLevel(in_edges_t, in_edges_s, coords.xy, 0.0, vec2<i32>(-1, 0)).g,
            textureSampleLevel(in_edges_t, in_edges_s, coords.xy, 0.0, vec2<i32>(0, -1)).r,
            textureSampleLevel(in_edges_t, in_edges_s, coords.zw, 0.0, vec2<i32>(1, 0)).gr
        );
        var cc = vec2<f32>(2.0) * c.xz + c.yw;
        cc = select(cc, vec2<f32>(0.0), d.zw >= vec2<f32>(0.9));
        weights += area_diag(d.xy, cc, subsample_indices.w).gr;
    }
    return weights;
}

// ── HORIZONTAL / VERTICAL ───────────────────────────────────────

// Length to add in the last step of a search (0, 1 or 2), from the bilinearly fetched edges
fn search_length(e: vec2<f32>, offset: f32) -> f32 {
    var scale = SMAA_SEARCHTEX_SIZE * vec2<f32>(0.5, -1.0);
    var bias = SMAA_SEARCHTEX_SIZE * vec2<f32>(offset, 1.0);
    scale += vec2<f32>(-1.0, 1.0);
    bias += vec2<f32>(0.5, -0.5);
    scale *= 1.0 / SMAA_SEARCHTEX_PACKED_SIZE;
    bias *= 1.0 / SMAA_SEARCHTEX_PACKED_SIZE;
    return textureSampleLevel(in_search_t, in_edges_s, scale * e + bias, 0.0).r;
}

// The coordinates are offset to sample between edges, fetching four of them at once (see @PSEUDO_GATHER4)
fn search_x_left(in_uv: vec2<f32>, end: f32) -> f32 {
    var uv = in_uv;
    var e = vec2<f32>(0.0, 1.0);
    while uv.x > end && e.g > 0.8281 && e.r == 0.0 {
        e = edges_at(uv);
        uv -= vec2<f32>(2.0, 0.0) * rt.xy;
    }
    let offset = -(255.0 / 127.0) * search_length(e, 0.0) + 3.25;
    return rt.x * offset + uv.x;
}

fn search_x_right(in_uv: vec2<f32>, end: f32) -> f32 {
    var uv = in_uv;
    var e = vec2<f32>(0.0, 1.0);
    while uv.x < end && e.g > 0.8281 && e.r == 0.0 {
        e = edges_at(uv);
        uv += vec2<f32>(2.0, 0.0) * rt.xy;
    }
    let offset = -(255.0 / 127.0) * search_length(e, 0.5) + 3.25;
    return -rt.x * offset + uv.x;
}

fn search_y_up(in_uv: vec2<f32>, end: f32) -> f32 {
    var uv = in_uv;
    var e = vec2<f32>(1.0, 0.0);
    while uv.y > end && e.r > 0.8281 && e.g == 0.0 {
        e = edges_at(uv);
        uv -= vec2<f32>(0.0, 2.0) * rt.xy;
    }
    let offset = -(255.0 / 127.0) * search_length(e.gr, 0.0) + 3.25;
    return rt.y * offset + uv.y;
}

fn search_y_down(in_uv: vec2<f32>, end: f32) -> f32 {
    var uv = in_uv;
    var e = vec2<f32>(1.0, 0.0);
    while uv.y < end && e.r > 0.8281 && e.g == 0.0 {
        e = edges_at(uv);
        uv += vec2<f32>(0.0, 2.0) * rt.xy;
    }
    let offset = -(255.0 / 127.0) * search_length(e.gr, 0.5) + 3.25;
    return -rt.y * offset + uv.y;
}

// Areas on each side of an edge, from its distances to its ends and its crossing edges
fn area(dist: vec2<f32>, e1: f32, e2: f32, offset: f32) -> vec2<f32> {
    var uv = SMAA_AREATEX_MAX_DISTANCE * round(4.0 * vec2<f32>(e1, e2)) + dist;
    uv = SMAA_AREATEX_PIXEL_SIZE * uv + 0.5 * SMAA_AREATEX_PIXEL_SIZE;
    uv.y += SMAA_AREATEX_SUBTEX_SIZE * offset;
    return textureSampleLevel(in_area_t, in_edges_s, uv, 0.0).rg;
}

// ── CORNERS ─────────────────────────────────────────────────────

fn detect_horizontal_corner_pattern(weights: vec2<f32>, uv: vec4<f32>, d: vec2<f32>) -> vec2<f32> {
    if in_params.smaa.w < 0.0 {
        return weights;
    }
    let left_right = step(d.xy, d.yx);
    // Less rounding for the pixels in the middle of a line
    let rounding = (1.0 - in_params.smaa.w) * left_right / (left_right.x + left_right.y);
    var factor = vec2<f32>(1.0);
    factor.x -= rounding.x * textureSampleLevel(in_edges_t, in_edges_s, uv.xy, 0.0, vec2<i32>(0, 1)).r;
    factor.x -= rounding.y * textureSampleLevel(in_edges_t, in_edges_s, uv.zw, 0.0, vec2<i32>(1, 1)).r;
    factor.y -= rounding.x * textureSampleLevel(in_edges_t, in_edges_s, uv.xy, 0.0, vec2<i32>(0, -2)).r;
    factor.y -= rounding.y * textureSampleLevel(in_edges_t, in_edges_s, uv.zw, 0.0, vec2<i32>(1, -2)).r;
    return weights * saturate(factor);
}

fn detect_vertical_corner_pattern(weights: vec2<f32>, uv: vec4<f32>, d: vec2<f32>) -> vec2<f32> {
    if in_params.smaa.w < 0.0 {
        return weights;
    }
    let left_right = step(d.xy, d.yx);
    let rounding = (1.0 - in_params.smaa.w) * left_right / (left_right.x + left_right.y);
    var factor = vec2<f32>(1.0);
    factor.x -= rounding.x * textureSampleLevel(in_edges_t, in_edges_s, uv.xy, 0.0, vec2<i32>(1, 0)).g;
    factor.x -= rounding.y * textureSampleLevel(in_edges_t, in_edges_s, uv.zw, 0.0, vec2<i32>(1, 1)).g;
    factor.y -= rounding.x * textureSampleLevel(in_edges_t, in_edges_s, uv.xy, 0.0, vec2<i32>(-2, 0)).g;
    factor.y -= rounding.y * textureSampleLevel(in_edges_t, in_edges_s, uv.zw, 0.0, vec2<i32>(-2, 1)).g;
    return weights * saturate(factor);
}

// ── MAIN ────────────────────────────────────────────────────────

@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    rt = smaa_rt_metrics(textureDimensions(in_edges_t));
    let uv = in.tex_coord;
    let position = in.clip_position.xy;
    let offset_0 = rt.xyxy * vec4<f32>(-0.25, -0.125, 1.25, -0.125) + uv.xyxy;
    let offset_1 = rt.xyxy * vec4<f32>(-0.125, -0.25, -0.125, 1.25) + uv.xyxy;
    // Ends of the searches
    let offset_2 = rt.xxyy * vec4<f32>(-2.0, 2.0, -2.0, 2.0) * in_params.smaa.y
        + vec4<f32>(offset_0.xz, offset_1.yw);
    let subsample_indices = vec4<f32>(0.0);

    var weights = vec4<f32>(0.0);
    let e = edges_at(uv);

    // Edge at north
    if e.g > 0.0 {
        // Diagonals have both north and west edges, and take priority
        if in_params.smaa.z > 0.0 {
            weights = vec4<f32>(calculate_diag_weights(uv, e, subsample_indices), weights.ba);
            if weights.r + weights.g != 0.0 {
                return weights;
            }
        }

        var coords: vec3<f32>;
        coords.x = search_x_left(offset_0.xy, offset_2.x);
        coords.y = offset_1.y;
        var d: vec2<f32>;
        d.x = coords.x;
        // Crossing edges sampled at -0.25 (@CROSSING_OFFSET) to tell them apart
        let e1 = textureSampleLevel(in_edges_t, in_edges_s, coords.xy, 0.0).r;
        coords.z = search_x_right(offset_0.zw, offset_2.y);
        d.y = coords.z;
        d = abs(round(rt.zz * d - position.xx));
        // The area texture is compressed quadratically
        let sqrt_d = sqrt(d);
        let e2 = textureSampleLevel(in_edges_t, in_edges_s, coords.zy, 0.0, vec2<i32>(1, 0)).r;
        weights = vec4<f32>(area(sqrt_d, e1, e2, subsample_indices.y), weights.ba);
        coords.y = uv.y;
        weights = vec4<f32>(detect_horizontal_corner_pattern(weights.rg, coords.xyzy, d), weights.ba);
    }

    // Edge at west
    if e.r > 0.0 {
        var coords: vec3<f32>;
        coords.y = search_y_up(offset_1.xy, offset_2.z);
        coords.x = offset_0.x;
        var d: vec2<f32>;
        d.x = coords.y;
        let e1 = textureSampleLevel(in_edges_t, in_edges_s, coords.xy, 0.0).g;
        coords.z = search_y_down(offset_1.zw, offset_2.w);
        d.y = coords.z;
        d = abs(round(rt.ww * d - position.yy));
        let sqrt_d = sqrt(d);
        let e2 = textureSampleLevel(in_edges_t, in_edges_s, coords.xz, 0.0, vec2<i32>(0, 1)).g;
        weights = vec4<f32>(weights.rg, area(sqrt_d, e1, e2, subsample_indices.x));
        coords.x = uv.x;
        weights = vec4<f32>(weights.rg, detect_vertical_corner_pattern(weights.ba, coords.xyxz, d));
    }
    return weights;
}
