struct ModelInput {
    @location(0) position:  vec3<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) normal:    vec3<f32>
};
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) tex_coord: vec2<f32>,
};

// From world space to normalized device coordinates
struct Camera {
    world_to_view: mat4x4<f32>,
    view_to_ndc: mat4x4<f32>
}
@group(0) @binding(0) var<uniform> in_camera: Camera;

@group(1) @binding(0) var in_heightmap: texture_2d<f32>;
@group(1) @binding(1) var in_heightmap_sampler: sampler;
@group(1) @binding(2) var in_normalmap: texture_2d<f32>;
@group(1) @binding(3) var in_normalmap_sampler: sampler;
@group(1) @binding(4) var in_splatmap_1: texture_2d<f32>;
@group(1) @binding(5) var in_splatmap_1_sampler: sampler;


@vertex
fn main(@builtin(instance_index) instance: u32, model: ModelInput) -> VertexOutput {
    var out: VertexOutput;

    let obj_to_world = mat4x4<f32>(
        vec4<f32>(1.0, 0.0, 0.0, 0.0),
        vec4<f32>(0.0, 1.0, 0.0, 0.0),
        vec4<f32>(0.0, 0.0, 1.0, 0.0),
        vec4<f32>(0.0, 0.0, 0.0, 1.0)
    );

    var world_pos = obj_to_world * vec4<f32>(model.position, 1.0);

    // Add some noise to the height based on the xz position
    let a = 6.0;
    let h = 20.0;
    world_pos.y = textureSampleLevel(in_heightmap, in_heightmap_sampler, model.tex_coord, 0.0).r * h - h / 2.0;

    let view_pos4 = in_camera.world_to_view
        * world_pos;
    let view_pos = view_pos4.xyz / view_pos4.w;
    out.clip_position = in_camera.view_to_ndc * vec4<f32>(view_pos, 1.0);

    // Compute normal with finite differences
    let delta = 0.1;
    let heightL = textureSampleLevel(in_heightmap, in_heightmap_sampler, (model.tex_coord + vec2<f32>(-delta, 0.0)), 0.0).r * h - h / 2.0;
    let heightR = textureSampleLevel(in_heightmap, in_heightmap_sampler, (model.tex_coord + vec2<f32>( delta, 0.0)), 0.0).r * h - h / 2.0;
    let heightD = textureSampleLevel(in_heightmap, in_heightmap_sampler, (model.tex_coord + vec2<f32>(0.0, -delta)), 0.0).r * h - h / 2.0;
    let heightU = textureSampleLevel(in_heightmap, in_heightmap_sampler, (model.tex_coord + vec2<f32>(0.0,  delta)), 0.0).r * h - h / 2.0;
    let normal = normalize(vec3<f32>(heightL - heightR, 2.0 * delta * 1e-1, heightD - heightU));
    out.normal = normal;

    out.tex_coord = model.tex_coord;

    return out;
}
