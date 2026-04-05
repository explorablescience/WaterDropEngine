struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>
};

// Multisampled pbr texture
@group(0) @binding(0) var pbr_texture: texture_multisampled_2d<f32>;
@group(0) @binding(1) var pbr_texture_sampler: sampler;

@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Use pixel coordinates directly from clip_position
    let pixel_coords = vec2<i32>(in.clip_position.xy);

    // Same for color
    var resolved_color = vec4<f32>(0.0);
    for (var sample: i32 = 0i; sample < 4i; sample = sample + 1i) {
        let sample_color = textureLoad(pbr_texture, pixel_coords, sample);
        resolved_color = resolved_color + sample_color;
    }
    return resolved_color / 4.0;
}
