struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) local: vec2<f32>,
    @location(1) @interpolate(flat) half_size: vec2<f32>,
    @location(2) @interpolate(flat) color: vec4<f32>,
    @location(3) @interpolate(flat) border_color: vec4<f32>,
    @location(4) @interpolate(flat) border: vec4<f32>,
    @location(5) @interpolate(flat) radius: f32,
    @location(6) @interpolate(flat) textured: f32,
    @location(7) uv: vec2<f32>,
};

// Image of textured quads, a placeholder otherwise
@group(1) @binding(0) var in_image: texture_2d<f32>;
@group(1) @binding(1) var in_image_sampler: sampler;

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    return select(pow((c + 0.055) / 1.055, vec3<f32>(2.4)), c / 12.92, c <= vec3<f32>(0.04045));
}

fn sd_rounded_box(p: vec2<f32>, half_size: vec2<f32>, radius: f32) -> f32 {
    let q = abs(p) - half_size + radius;
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - radius;
}

@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    let radius = min(in.radius, min(in.half_size.x, in.half_size.y));
    let outer = clamp(0.5 - sd_rounded_box(in.local, in.half_size, radius), 0.0, 1.0);

    // Sampled out of the branch, which is not in uniform control flow
    let image = textureSample(in_image, in_image_sampler, in.uv);
    var color = in.color;
    if (in.textured > 0.5) {
        color = in.color * vec4<f32>(srgb_to_linear(image.rgb), image.a);
    }
    let max_border = max(max(in.border.x, in.border.y), max(in.border.z, in.border.w));
    if (max_border > 0.0) {
        // Inner box, inset by each border width
        let inner_min = -in.half_size + in.border.xy;
        let inner_max = in.half_size - in.border.zw;
        let inner_half = max((inner_max - inner_min) * 0.5, vec2<f32>(0.0));
        let inner_center = (inner_min + inner_max) * 0.5;
        let inner_radius = max(radius - max_border, 0.0);
        let inner = clamp(0.5 - sd_rounded_box(in.local - inner_center, inner_half, inner_radius), 0.0, 1.0);
        color = mix(in.border_color, in.color, inner);
    }
    return vec4<f32>(color.rgb, color.a * outer);
}
