#include "core/render/pbr/pbr_functions.wgsl"

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,  // Clip space position (after projection)
    @location(0) tex_coord:    vec2<f32>,     // Texture coordinates (UV)
    @location(1) normal_world: vec3<f32>,     // Normal in world space
    @location(2) tangent_world: vec4<f32>,    // Tangent in world space
    @location(3) bitangent_world: vec3<f32>,  // Bitangent in world space
    @location(4) ndc_z: f32,                  // Linear view-space depth (-view_z, always positive)
};

// Push constants informations about the current mesh
struct PushConstants {
    first_vertex: u32,
    first_index: u32
}
var<push_constant> push_constants: PushConstants;

// List of vertices and indices from the ssbo
// CPU Vertex layout: position[3], uv[2], normal[3], tangent[4]
// Storage buffer requires vec3 aligned to 16 bytes in arrays
// So we read as: vec4 (position + pad), vec2 (uv) + vec2 (normal.xy), vec2 (normal.z + pad) + vec2 (tangent.xy), vec2 (tangent.zw)
// Actually, simpler: just use proper size with padding
struct VertexData {
    position:  vec3<f32>,    // 12 bytes
    uv_x:      f32,          // 4 bytes (total 16)
    uv_y:      f32,          // 4 bytes
    normal_x:  f32,          // 4 bytes
    normal_y:  f32,          // 4 bytes (total 16)
    normal_z:  f32,          // 4 bytes
    tangent:   vec3<f32>,    // 12 bytes (total 16)
    tangent_w: f32,          // 4 bytes (total 16)
}
@group(0) @binding(0) var<storage, read> in_vertices: array<VertexData>;
@group(0) @binding(1) var<storage, read> in_indices: array<u32>;

@group(1) @binding(0) var<uniform> in_camera: Camera;

// Object to world space transformation ssbo
struct ObjectToWorld {
    obj_to_world:  mat4x4<f32>
}
@group(2) @binding(0) var<storage, read> in_raw_transform: array<ObjectToWorld>;
@group(2) @binding(1) var<storage, read> in_instance_to_transform: array<u32>;

@vertex
fn main(@builtin(instance_index) instance: u32, @builtin(vertex_index) vid: u32) -> VertexOutput {
    var out: VertexOutput;

    // Fetch vertices and indices
    let index = in_indices[vid + push_constants.first_index];
    let vertex = in_vertices[index + push_constants.first_vertex];

    // Reconstruct vectors from individual components
    let tex_coord = vec2<f32>(vertex.uv_x, vertex.uv_y);
    let normal = vec3<f32>(vertex.normal_x, vertex.normal_y, vertex.normal_z);
    let tangent = vec4<f32>(vertex.tangent.x, vertex.tangent.y, vertex.tangent.z, vertex.tangent_w);

    // Transform position to clip space
    let obj_to_world = in_raw_transform[in_instance_to_transform[instance]].obj_to_world;
    let view_pos4 = in_camera.world_to_view
        * obj_to_world
        * vec4<f32>(vertex.position, 1.0);
    let clip_pos = in_camera.view_to_ndc * view_pos4;
    out.clip_position = clip_pos;
    out.ndc_z = -view_pos4.z;

    // Pass through texture coordinates
    out.tex_coord = tex_coord;

    // Transform normal and tangent to world space
    let nm = normal_matrix(obj_to_world);
    out.normal_world = nm * normal;
    out.tangent_world = vec4<f32>(nm * tangent.xyz, tangent.w);
    out.bitangent_world = cross(out.normal_world, out.tangent_world.xyz) * out.tangent_world.w;

    return out;
}
