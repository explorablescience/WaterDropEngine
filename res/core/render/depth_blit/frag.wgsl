struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>
};

// Multisampled depth texture
@group(0) @binding(0) var ms_depth: texture_multisampled_2d<f32>;
@group(0) @binding(1) var ms_depth_sampler: sampler;

@fragment
fn main(in: VertexOutput) -> @builtin(frag_depth) f32 {
    // Use pixel coordinates directly from clip_position
    let pixel_coords = vec2<i32>(in.clip_position.xy);
    
    // Resolve MSAA depth by averaging all samples for smooth blitting
    var resolved_depth: f32 = 0.0;
    for (var sample: i32 = 0i; sample < 4i; sample = sample + 1i) {
        let sample_depth: f32 = textureLoad(ms_depth, pixel_coords, sample).r;
        resolved_depth = resolved_depth + sample_depth;
    }
    return resolved_depth * 0.25;
}
