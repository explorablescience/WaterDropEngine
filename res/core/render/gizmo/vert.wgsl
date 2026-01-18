struct ModelInput {
    @location(0) position:  vec3<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) normal:    vec3<f32>
};
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>
};

// From world space to normalized device coordinates
struct Camera {
    world_to_ndc: mat4x4<f32>
}
@group(0) @binding(0) var<uniform> in_camera: Camera;

// Object to world space transformation ssbo
struct ObjectToWorld {
    obj_to_world:  mat4x4<f32>
}
@group(1) @binding(0) var<storage> in_model: array<ObjectToWorld>;


@vertex
fn main(@builtin(instance_index) instance: u32, model: ModelInput) -> VertexOutput {
    var out: VertexOutput;

    let obj_to_world = in_model[instance].obj_to_world;
    out.clip_position = in_camera.world_to_ndc
        * obj_to_world
        * vec4<f32>(model.position, 1.0);

    return out;
}
