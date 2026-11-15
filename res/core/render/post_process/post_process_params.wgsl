// Parameters of the post-processing stack (see PostProcessUniform).
struct PostProcessParams {
    bloom: vec4<f32>,          // x = threshold, y = knee, z = scatter, w = intensity
    white_balance: mat3x3<f32>,
    grading: vec4<f32>,        // x = exposure multiplier, y = contrast, z = saturation, w = tonemapper
    color_filter: vec4<f32>,
    lift: vec4<f32>,
    gamma: vec4<f32>,
    gain: vec4<f32>,
    vignette: vec4<f32>,       // x = intensity, y = radius, z = smoothness, w = roundness
    vignette_color: vec4<f32>,
    effects: vec4<f32>,        // x = chromatic aberration, y = grain intensity, z = grain response, w = dithering
    time: vec4<f32>            // x = time in seconds
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>
};

fn luminance(color: vec3<f32>) -> f32 {
    return dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
}
