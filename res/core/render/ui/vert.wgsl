struct UiQuad {
    rect: vec4<f32>, // top-left xy, size zw (physical pixels)
    color: vec4<f32>,
    border_color: vec4<f32>,
    border: vec4<f32>, // left, top, right, bottom
    radius: vec4<f32>,
};
struct UiQuads {
    viewport: vec4<f32>,
    quads: array<UiQuad>,
};
@group(0) @binding(0) var<storage, read> in_ui: UiQuads;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) local: vec2<f32>, // pixel position relative to the quad center
    @location(1) @interpolate(flat) half_size: vec2<f32>,
    @location(2) @interpolate(flat) color: vec4<f32>,
    @location(3) @interpolate(flat) border_color: vec4<f32>,
    @location(4) @interpolate(flat) border: vec4<f32>,
    @location(5) @interpolate(flat) radius: f32,
    @location(6) @interpolate(flat) textured: f32,
    @location(7) uv: vec2<f32>,
};

@vertex
fn main(@builtin(vertex_index) vid: u32, @builtin(instance_index) iid: u32) -> VertexOutput {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0)
    );
    let quad = in_ui.quads[iid];
    let uv = corners[vid];
    let pixel = quad.rect.xy + uv * quad.rect.zw;
    let ndc = pixel / in_ui.viewport.xy * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0);

    var out: VertexOutput;
    out.clip_position = vec4<f32>(ndc, 0.0, 1.0);
    out.local = (uv - 0.5) * quad.rect.zw;
    out.half_size = quad.rect.zw * 0.5;
    out.color = quad.color;
    out.border_color = quad.border_color;
    out.border = quad.border;
    out.radius = quad.radius.x;
    out.textured = quad.radius.y;
    out.uv = uv;
    return out;
}
