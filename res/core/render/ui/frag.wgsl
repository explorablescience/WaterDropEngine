struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) local: vec2<f32>,
    @location(1) @interpolate(flat) half_size: vec2<f32>,
    @location(2) @interpolate(flat) color: vec4<f32>,
    @location(3) @interpolate(flat) border_color: vec4<f32>,
    @location(4) @interpolate(flat) border: vec4<f32>,
    @location(5) @interpolate(flat) radius: f32,
};

fn sd_rounded_box(p: vec2<f32>, half_size: vec2<f32>, radius: f32) -> f32 {
    let q = abs(p) - half_size + radius;
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - radius;
}

@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    let radius = min(in.radius, min(in.half_size.x, in.half_size.y));
    let outer = clamp(0.5 - sd_rounded_box(in.local, in.half_size, radius), 0.0, 1.0);

    var color = in.color;
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
