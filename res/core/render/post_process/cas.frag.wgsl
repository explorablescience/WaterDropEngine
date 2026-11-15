// AMD FidelityFX contrast adaptive sharpening: sharpens each pixel against its 3x3 neighborhood,
// less where the neighborhood is already contrasted. Copies the image when it is disabled.
#include "core/render/post_process/post_process_params.wgsl"

@group(0) @binding(0) var in_image_t: texture_2d<f32>;
@group(0) @binding(1) var<uniform> in_params: PostProcessParams;

fn load(pixel: vec2<i32>, offset: vec2<i32>, size: vec2<i32>) -> vec3<f32> {
    return textureLoad(in_image_t, clamp(pixel + offset, vec2<i32>(0), size - 1), 0).rgb;
}

@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(in.clip_position.xy);
    let size = vec2<i32>(textureDimensions(in_image_t));
    let e = load(pixel, vec2<i32>(0, 0), size);
    if in_params.sharpening.x != 1.0 {
        return vec4<f32>(e, 1.0);
    }

    // a b c
    // d e f
    // g h i
    let a = load(pixel, vec2<i32>(-1, -1), size);
    let b = load(pixel, vec2<i32>(0, -1), size);
    let c = load(pixel, vec2<i32>(1, -1), size);
    let d = load(pixel, vec2<i32>(-1, 0), size);
    let f = load(pixel, vec2<i32>(1, 0), size);
    let g = load(pixel, vec2<i32>(-1, 1), size);
    let h = load(pixel, vec2<i32>(0, 1), size);
    let i = load(pixel, vec2<i32>(1, 1), size);

    // Soft min and max, in [0, 2]: the cross plus the whole neighborhood
    var mn = min(min(min(d, e), min(f, b)), h);
    var mx = max(max(max(d, e), max(f, b)), h);
    mn += min(mn, min(min(a, c), min(g, i)));
    mx += max(mx, max(max(a, c), max(g, i)));

    // Amplitude, decreasing with the distance to the black and white limits
    let amp = sqrt(clamp(min(mn, 2.0 - mx) / max(mx, vec3<f32>(1e-5)), vec3<f32>(0.0), vec3<f32>(1.0)));
    let w = amp * in_params.sharpening.y;
    let color = (w * (b + d + f + h) + e) / (1.0 + 4.0 * w);
    return vec4<f32>(clamp(color, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}
