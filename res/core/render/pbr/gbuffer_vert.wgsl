struct ModelInput {
    @location(0) position:  vec3<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) normal:    vec3<f32>,
    @location(3) tangent:   vec4<f32>,
};
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,  // Clip space position (after projection)
    @location(0) tex_coord:    vec2<f32>,     // Texture coordinates (UV)
    @location(1) normal_world: vec3<f32>,     // Normal in world space
    @location(2) tangent_world: vec4<f32>,    // Tangent in world space
    @location(3) bitangent_world: vec3<f32>,  // Bitangent in world space
    @location(4) view_z: f32,                 // View-space Z (linear depth)
};

// From world space to normalized device coordinates
struct Camera {
    world_to_view: mat4x4<f32>,
    view_to_ndc: mat4x4<f32>
}
@group(0) @binding(0) var<uniform> in_camera: Camera;

// Object to world space transformation ssbo
struct ObjectToWorld {
    obj_to_world:  mat4x4<f32>
}
@group(1) @binding(0) var<storage, read> in_raw_transform: array<ObjectToWorld>;
@group(1) @binding(1) var<storage, read> in_instance_to_transform: array<u32>;

// Function to compute the inverse of a 3x3 matrix
fn inverse(m: mat3x3<f32>) -> mat3x3<f32> {
    let a = m[0];
    let b = m[1];
    let c = m[2];

    let r0 = cross(b, c);
    let r1 = cross(c, a);
    let r2 = cross(a, b);

    let inv_det = 1.0 / dot(r2, c);

    return mat3x3<f32>(
        r0 * inv_det,
        r1 * inv_det,
        r2 * inv_det
    );
}

@vertex
fn main(@builtin(instance_index) instance: u32, model: ModelInput) -> VertexOutput {
    var out: VertexOutput;

    // Transform position to clip space
    let obj_to_world = in_raw_transform[in_instance_to_transform[instance]].obj_to_world;
    let view_pos4 = in_camera.world_to_view
        * obj_to_world
        * vec4<f32>(model.position, 1.0);
    let view_pos = view_pos4.xyz / view_pos4.w;
    out.clip_position = in_camera.view_to_ndc * vec4<f32>(view_pos, 1.0);
    
    // Store linear view-space depth (negative Z in view space)
    out.view_z = -view_pos.z;

    // Pass through texture coordinates
    out.tex_coord = model.tex_coord;

    // Transform normal to world space
    let normal_matrix = transpose(inverse(mat3x3<f32>(
        obj_to_world[0].xyz,
        obj_to_world[1].xyz,
        obj_to_world[2].xyz
    )));
    out.normal_world = normal_matrix * model.normal;

    // Transform tangent to world space
    out.tangent_world = vec4<f32>(
        normal_matrix * model.tangent.xyz,
        model.tangent.w
    );

    // Transform bitangent to world space
    out.bitangent_world = cross(out.normal_world, out.tangent_world.xyz) * out.tangent_world.w;

    return out;
}
