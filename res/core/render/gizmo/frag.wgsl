struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>
};

@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
