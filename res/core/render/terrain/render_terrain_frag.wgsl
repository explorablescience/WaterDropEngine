struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>
};

// Camera uniform buffer
struct Camera {
    world_to_view: mat4x4<f32>,
    view_to_ndc: mat4x4<f32>,
    ndc_to_view: mat4x4<f32>,
    view_to_world: mat4x4<f32>,
    position: vec4<f32>
}
@group(0) @binding(0) var<uniform> in_camera: Camera;




// ======== MAIN FRAGMENT SHADER ========
@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(vec3<f32>(0.8, 0.2, 0.2), 1.0);
}
