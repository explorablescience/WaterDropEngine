// Maps a texel of the viewed texture to the display (see DebugViewParams).
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>
};

struct DebugViewParams {
    range: vec2<f32>,
    exposure: f32,
    channels: u32, // 0 = RGB, 1 = R, 2 = G, 3 = B, 4 = A
    mode: u32,     // 0 = data, 1 = color, 2 = integer
    tonemap: u32,
    _pad: vec2<u32>
};
var<immediate> params: DebugViewParams;

fn texel(uv: vec2<f32>, size: vec2<u32>) -> vec2<u32> {
    return min(vec2<u32>(uv * vec2<f32>(size)), size - 1u);
}

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    return select(pow((c + 0.055) / 1.055, vec3<f32>(2.4)), c / 12.92, c <= vec3<f32>(0.04045));
}

// Same as debug_integer_color in render_graph_debug.rs
fn integer_color(value: u32) -> vec3<f32> {
    if value == 0u {
        return vec3<f32>(0.0);
    }
    let hue = fract(f32(value - 1u) * 0.618034);
    let ramp = clamp(abs(fract(hue + vec3<f32>(0.0, 2.0 / 3.0, 1.0 / 3.0)) * 6.0 - 3.0) - 1.0, vec3<f32>(0.0), vec3<f32>(1.0));
    return mix(vec3<f32>(1.0), ramp, 0.8);
}

fn display(value: vec4<f32>) -> vec4<f32> {
    var c = value.rgb;
    switch params.channels {
        case 1u: { c = vec3<f32>(value.r); }
        case 2u: { c = vec3<f32>(value.g); }
        case 3u: { c = vec3<f32>(value.b); }
        case 4u: { c = vec3<f32>(value.a); }
        default: {}
    }
    // NaNs in magenta, infinities in cyan
    if any(c != c) {
        return vec4<f32>(1.0, 0.0, 1.0, 1.0);
    }
    if any(abs(c) > vec3<f32>(3.0e38)) {
        return vec4<f32>(0.0, 1.0, 1.0, 1.0);
    }
    if params.mode == 2u {
        return vec4<f32>(integer_color(u32(max(round(c.r), 0.0))), 1.0);
    }
    c = (c - params.range.x) / max(params.range.y - params.range.x, 1e-20);

    // Colors are linear, encoded by the sRGB target. Data is shown as is.
    if params.mode == 1u {
        c = max(c, vec3<f32>(0.0)) * params.exposure;
        if params.tonemap == 1u {
            c = c / (1.0 + c);
        }
        return vec4<f32>(c, 1.0);
    }
    return vec4<f32>(srgb_to_linear(clamp(c, vec3<f32>(0.0), vec3<f32>(1.0))), 1.0);
}
