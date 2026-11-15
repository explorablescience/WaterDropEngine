// ================================================================
// Screen-space ambient occlusion (see AmbientOcclusionSettings).
// Include with:  #include "core/render/pbr/ambient_occlusion.wgsl"
// ================================================================

struct AmbientOcclusionParams {
    effect: vec4<f32>,  // x = radius (world units), y = falloff (fraction of the radius), z = intensity, w = enabled
    quality: vec4<f32>, // x = slices, y = steps per side, z = max screen radius (pixels), w = blur sharpness
    debug: vec4<f32>    // x = debug view
}

// Relative depth difference under which the four half resolution texels around a pixel are on its surface
const OCCLUSION_UPSAMPLE_TOLERANCE: f32 = 0.05;

// Visibility of a full resolution pixel from the half resolution occlusion (r = visibility, g = linear depth), where
// texel i was computed at the pixel 2i. Bilinear on continuous surfaces, the closest depth across edges.
fn upsample_occlusion(occlusion: texture_2d<f32>, pixel: vec2<i32>, depth: f32) -> f32 {
    let size = vec2<i32>(textureDimensions(occlusion));
    let position = vec2<f32>(pixel) * 0.5;
    let base = vec2<i32>(floor(position));
    let f = position - vec2<f32>(base);

    var bilinear = 0.0;
    var max_diff = 0.0;
    var closest = 1.0;
    var closest_diff = 1e30;
    for (var i = 0; i < 4; i++) {
        let offset = vec2<i32>(i & 1, i >> 1u);
        let w = select(1.0 - f.x, f.x, offset.x == 1) * select(1.0 - f.y, f.y, offset.y == 1);
        if w <= 0.0 {
            continue;
        }
        let texel = textureLoad(occlusion, min(base + offset, size - 1), 0).rg;
        // No geometry at this texel
        let diff = select(1e30, abs(texel.y - depth), texel.y > 0.0);
        bilinear += texel.x * w;
        max_diff = max(max_diff, diff);
        if diff < closest_diff {
            closest_diff = diff;
            closest = texel.x;
        }
    }
    if max_diff < OCCLUSION_UPSAMPLE_TOLERANCE * depth {
        return bilinear;
    }
    return select(1.0, closest, closest_diff < 1e30);
}
