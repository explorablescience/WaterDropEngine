#include "core/render/pbr/pbr_functions.wgsl"

// Shadow map vertex shader.
// Transforms each vertex to the light's clip space for depth-only rendering.

struct VertexData {
    position:  vec3<f32>,
    uv_x:      f32,
    uv_y:      f32,
    normal_x:  f32,
    normal_y:  f32,
    normal_z:  f32,
    tangent:   vec3<f32>,
    tangent_w: f32
}

// Group 0: global mesh SSBO (reuses SsboMeshBinding layout)
@group(0) @binding(0) var<storage, read> in_vertices: array<VertexData>;
@group(0) @binding(1) var<storage, read> in_indices:  array<u32>;

// Group 1: shadow params (light view-projection matrices for 3 cascades)
@group(1) @binding(0) var<uniform> in_shadow_params: array<ShadowParams, 3>;

// Group 2: per-instance transforms (reuses SsboTransformBinding layout)
struct ObjectToWorld {
    obj_to_world: mat4x4<f32>
}
@group(2) @binding(0) var<storage, read> in_raw_transform:        array<ObjectToWorld>;
@group(2) @binding(1) var<storage, read> in_instance_to_transform: array<u32>;

struct PushConstants {
    first_vertex:  u32,
    first_index:   u32,
    cascade_index: u32
}
var<push_constant> push_constants: PushConstants;

@vertex
fn main(
    @builtin(instance_index) instance: u32,
    @builtin(vertex_index)   vid:      u32
) -> @builtin(position) vec4<f32> {
    let index  = in_indices[vid + push_constants.first_index];
    let vertex = in_vertices[index + push_constants.first_vertex];
    let obj_to_world = in_raw_transform[in_instance_to_transform[instance]].obj_to_world;
    let world_pos = obj_to_world * vec4<f32>(vertex.position, 1.0);
    return in_shadow_params[push_constants.cascade_index].view_proj * world_pos;
}
