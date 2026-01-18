struct ModelInput {
    @location(0) position:  vec3<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) normal:    vec3<f32>
};
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) position: vec3<f32>,
    @location(1) @interpolate(flat) normal: vec3<f32>
};

// From world space to normalized device coordinates
struct Camera {
    world_to_ndc: mat4x4<f32>,
    ndc_to_world: mat4x4<f32>,
    position: vec4<f32>
}
@group(0) @binding(0) var<uniform> in_camera: Camera;

@vertex
fn main(model: ModelInput) -> VertexOutput {
    var out: VertexOutput;

    out.clip_position = in_camera.world_to_ndc
        * vec4<f32>(model.position, 1.0);
    out.position = model.position;
    out.normal = model.normal;

    return out;
}
