struct Camera {
    world_to_view: mat4x4<f32>,
    view_to_ndc: mat4x4<f32>,
};
@group(0) @binding(0) var<uniform> in_camera: Camera;

struct GizmoVertex {
    position: vec4<f32>,
    color: vec4<f32>,
};
@group(1) @binding(0) var<storage, read> in_vertices: array<GizmoVertex>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>
};

@vertex
fn main(@builtin(vertex_index) vid: u32) -> VertexOutput {
    let vertex = in_vertices[vid];

    let view_pos = in_camera.world_to_view * vec4<f32>(vertex.position.xyz, 1.0);

    var out: VertexOutput;
    out.clip_position = in_camera.view_to_ndc * view_pos;
    out.color = vertex.color;
    return out;
}
