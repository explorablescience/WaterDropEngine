struct SelectionAreaMaterial {
    color: vec4<f32>,
};
@group(3) @binding(0) var<uniform> in_selection_area_material: SelectionAreaMaterial;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) world_pos: vec3<f32>,
};

// Ground selection rectangle: translucent hatched fill, glow toward the edges, a crisp border and
// brighter corner brackets. Widths are in pixels (from the UV screen derivatives), so the frame
// stays sharp whatever the rectangle size or zoom.
@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = in_selection_area_material.color;
    let px = max(fwidth(in.uv), vec2<f32>(1e-6));

    // Distance to the nearest edge, in pixels, along each axis
    let edge = min(in.uv, 1.0 - in.uv) / px;
    let d = min(edge.x, edge.y);

    let border = 1.0 - smoothstep(1.5, 2.5, d);
    let in_corner = step(edge.x, 22.0) * step(edge.y, 22.0);
    let bracket = in_corner * (1.0 - smoothstep(3.0, 4.0, d));
    let glow = exp(-d / 24.0);

    // Diagonal hatching in world space
    let stripes = step(0.5, fract((in.world_pos.x + in.world_pos.z) * 2.0));

    let fill = color.a * (0.6 + 0.25 * stripes) + glow * color.a * 1.2;
    let alpha = clamp(max(fill, max(border * 0.85, bracket)), 0.0, 1.0);
    let rgb = mix(color.rgb, vec3<f32>(1.0), bracket * 0.6 + border * 0.2);
    return vec4<f32>(rgb, alpha);
}
