struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>
};

// The texture to display
@group(0) @binding(0) var texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sam: sampler;

@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    let coords = vec2<f32>(in.tex_coord.x, 1.0 - in.tex_coord.y);
    let val = textureSample(texture, texture_sam, coords).rgb;
    return vec4<f32>(val, 1.0);
}
