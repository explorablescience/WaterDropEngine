struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>
};

struct FragmentOutput {
    @builtin(frag_depth) depth: f32,
    @location(0) color: vec4<f32>
};

// Multisampled pbr texture
@group(0) @binding(0) var pbr_texture: texture_multisampled_2d<f32>;
@group(0) @binding(1) var pbr_texture_sampler: sampler;
// Multisampled depth texture
@group(1) @binding(0) var ms_depth: texture_multisampled_2d<f32>;
@group(1) @binding(1) var ms_depth_sampler: sampler;

@fragment
fn main(in: VertexOutput) -> FragmentOutput {
    var out: FragmentOutput;

    // Use pixel coordinates directly from clip_position
    let pixel_coords = vec2<i32>(in.clip_position.xy);
    
    // Resolve MSAA depth by averaging all samples for smooth blitting
    var resolved_depth: f32 = 0.0;
    for (var sample: i32 = 0i; sample < 4i; sample = sample + 1i) {
        let sample_depth: f32 = textureLoad(ms_depth, pixel_coords, sample).r;
        resolved_depth = resolved_depth + sample_depth;
    }
    out.depth = resolved_depth / 4.0;

    // Same for color
    var resolved_color: vec4<f32> = vec4<f32>(0.0);
    for (var sample: i32 = 0i; sample < 4i; sample = sample + 1i) {
        let sample_color: vec4<f32> = textureLoad(pbr_texture, pixel_coords, sample);
        resolved_color = resolved_color + sample_color;
    }
    out.color = resolved_color / 4.0;

    return out;
}
