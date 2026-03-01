struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) tex_coord: vec2<f32>,
};

@group(1) @binding(0) var in_heightmap: texture_2d<f32>;
@group(1) @binding(1) var in_heightmap_sampler: sampler;
@group(1) @binding(2) var in_normalmap: texture_2d<f32>;
@group(1) @binding(3) var in_normalmap_sampler: sampler;
@group(1) @binding(4) var in_splatmap_1: texture_2d<f32>;
@group(1) @binding(5) var in_splatmap_1_sampler: sampler;

@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    let sun_dir = normalize(vec3<f32>(0.5, 1.0, 0.5));
    let light = max(dot(in.normal, sun_dir), 0.0);
    let color = mix(vec3<f32>(0.1, 0.4, 0.1), vec3<f32>(0.2, 0.7, 0.2), light);

    // return vec4<f32>(color, 1.0);
    // return vec4<f32>(vec3<f32>(in.tex_coord, 0.0), 1.0);
    return vec4<f32>(textureSample(in_heightmap, in_heightmap_sampler, in.tex_coord).rgb, 1.0);
    // return vec4<f32>(textureSample(in_heightmap, in_heightmap_sampler, in.tex_coord).rgb, 1.0);
}
