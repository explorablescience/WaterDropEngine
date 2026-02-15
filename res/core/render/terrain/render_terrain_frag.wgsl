struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
};

@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    let sun_dir = normalize(vec3<f32>(0.5, 1.0, 0.5));
    let light = max(dot(in.normal, sun_dir), 0.0);
    let color = mix(vec3<f32>(0.1, 0.4, 0.1), vec3<f32>(0.2, 0.7, 0.2), light);
    return vec4<f32>(color, 1.0);
}
