struct PushConstants {
    first_vertex: u32,
    first_index: u32,
    transform_id: u32,
};
var<push_constant> push_constants: PushConstants;

struct VertexData {
    position: vec3<f32>,
    uv_x: f32,
    uv_y: f32,
    normal_x: f32,
    normal_y: f32,
    normal_z: f32,
    tangent: vec3<f32>,
    tangent_w: f32,
};
@group(0) @binding(0) var<storage, read> in_vertices: array<VertexData>;
@group(0) @binding(1) var<storage, read> in_indices: array<u32>;

struct Camera {
    world_to_view: mat4x4<f32>,
    view_to_ndc: mat4x4<f32>,
};
@group(1) @binding(0) var<uniform> in_camera: Camera;

struct ObjectToWorld {
    obj_to_world: mat4x4<f32>,
};
@group(2) @binding(0) var<storage, read> in_raw_transform: array<ObjectToWorld>;

struct OutlineMaterial {
    color: vec4<f32>,
    thickness: vec4<f32>,
};
@group(3) @binding(0) var<uniform> in_outline_material: OutlineMaterial;

@vertex
fn main(@builtin(instance_index) instance: u32, @builtin(vertex_index) vid: u32) -> @builtin(position) vec4<f32> {
    let index = in_indices[vid + push_constants.first_index];
    let vertex = in_vertices[index + push_constants.first_vertex];

    // Build an expanded shell in object space for the outline pass.
    let outline_scale = 1.0 + max(in_outline_material.thickness.x, 0.0);
    let scaled_position = vertex.position * outline_scale;

    let obj_to_world = in_raw_transform[push_constants.transform_id].obj_to_world;
    let world_pos = obj_to_world * vec4<f32>(scaled_position, 1.0);
    let view_pos = in_camera.world_to_view * world_pos;

    return in_camera.view_to_ndc * view_pos;
}
