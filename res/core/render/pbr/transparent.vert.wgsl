#include "core/render/pbr/pbr_functions.wgsl"

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coord:    vec2<f32>,
    @location(1) normal_world: vec3<f32>,
    @location(2) world_pos:    vec3<f32>,
};

struct PushConstants {
    first_vertex: u32,
    first_index: u32
}
var<push_constant> push_constants: PushConstants;

// Same layout as gbuffer.vert.wgsl
struct VertexData {
    position:  vec3<f32>,
    uv_x:      f32,
    uv_y:      f32,
    normal_x:  f32,
    normal_y:  f32,
    normal_z:  f32,
    tangent:   vec3<f32>,
    tangent_w: f32,
}
@group(0) @binding(0) var<storage, read> in_vertices: array<VertexData>;
@group(0) @binding(1) var<storage, read> in_indices: array<u32>;

@group(1) @binding(0) var<uniform> in_camera: Camera;

struct ObjectToWorld {
    obj_to_world:  mat4x4<f32>
}
@group(2) @binding(0) var<storage, read> in_raw_transform: array<ObjectToWorld>;
@group(2) @binding(1) var<storage, read> in_instance_to_transform: array<u32>;

@vertex
fn main(@builtin(instance_index) instance: u32, @builtin(vertex_index) vid: u32) -> VertexOutput {
    var out: VertexOutput;

    let index = in_indices[vid + push_constants.first_index];
    let vertex = in_vertices[index + push_constants.first_vertex];

    let obj_to_world = in_raw_transform[in_instance_to_transform[instance]].obj_to_world;
    let world_pos4 = obj_to_world * vec4<f32>(vertex.position, 1.0);
    out.clip_position = in_camera.view_to_ndc * in_camera.world_to_view * world_pos4;
    out.world_pos = world_pos4.xyz;
    out.tex_coord = vec2<f32>(vertex.uv_x, vertex.uv_y);
    out.normal_world = normal_matrix(obj_to_world) * vec3<f32>(vertex.normal_x, vertex.normal_y, vertex.normal_z);

    return out;
}
