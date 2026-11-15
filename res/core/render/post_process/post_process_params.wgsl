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
    time: vec4<f32>,           // x = time in seconds
    anti_aliasing: vec4<f32>,  // x = method (0 = none, 1 = FXAA)
    exposure: vec4<f32>,       // x = eye adaptation, y = delta time, z = brighten speed, w = darken speed
    exposure_metering: vec4<f32>, // x = min log2 luminance, y = log2 luminance range, z = low percentile, w = high percentile
    exposure_limits: vec4<f32> // x = min EV, y = max EV
};

// Eye adaptation state, kept on the GPU between frames
struct ExposureState {
    ev: f32,
    initialized: u32
};

const EXPOSURE_HISTOGRAM_BINS: u32 = 64u;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>
};

fn luminance(color: vec3<f32>) -> f32 {
    return dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
}
