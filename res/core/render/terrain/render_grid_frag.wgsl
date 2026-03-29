struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_xz: vec2<f32>, // world-space XZ of fragment
    @location(1) chunk_uv: vec2<f32>  // [0,1] within the matched chunk
};

// Chunk description buffers
struct ChunkDescription {
    chunk_size:        f32,
    subdivisions:      f32,
    major_line_width:  f32,  // world units, e.g. 0.05
    minor_line_width:  f32,  // world units, e.g. 0.01
    major_color:       vec4<f32>,
    minor_color:       vec4<f32>,
    fade_center:       vec2<f32>,  // world position of the center point for fading
    fade_start:        f32,        // world distance from center point to start fading
    fade_end:          f32         // world distance from center point to end fading
}
@group(1) @binding(0) var<uniform> in_grid: ChunkDescription;
struct ChunkPos {
    xz: vec2<f32>,
}
@group(1) @binding(1) var<storage, read> in_chunk_positions: array<ChunkPos>;


// Returns how strongly a coordinate is "on" a grid line.
// coord: position in grid space (e.g. 0..subdivisions)
// line_width: line half-width in grid units
fn grid_line_alpha(coord: f32, line_half_width: f32) -> f32 {
    // Distance from coord to nearest integer
    let d = abs(fract(coord + 0.5) - 0.5); // 0 = on line, 0.5 = between lines
    // Smooth falloff: 1.0 at line center, 0.0 at line_half_width away
    return 1.0 - smoothstep(0.0, line_half_width, d);
}

@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Computed parameters
    let subdivisions_sqrt = sqrt(in_grid.subdivisions);
    let major_ratio = 2.0;
    let cell_size = in_grid.chunk_size / 2.0;
    let sub_coord = in.chunk_uv * subdivisions_sqrt * 2.0;

    // Compute screen-space derivative for antialiasing width
    let ddx = dpdx(sub_coord);
    let ddy = dpdy(sub_coord);
    let fwidth_sub = max(length(ddx), length(ddy)); // ~1 pixel in sub_coord space

    // --- Minor lines (subdivisions) ---
    let minor_hw = (in_grid.minor_line_width / cell_size) * 0.5 + fwidth_sub;
    let minor_x = grid_line_alpha(sub_coord.x, minor_hw);
    let minor_y = grid_line_alpha(sub_coord.y, minor_hw);
    let minor_mask = max(minor_x, minor_y);

    // --- Major lines (chunk boundaries) ---
    let major_hw = ((in_grid.major_line_width / cell_size) * 0.5 + fwidth_sub) / subdivisions_sqrt * major_ratio * 1.3;
    let major_x = grid_line_alpha(sub_coord.x / subdivisions_sqrt * major_ratio, major_hw);
    let major_y = grid_line_alpha(sub_coord.y / subdivisions_sqrt * major_ratio, major_hw);
    let major_mask = max(major_x, major_y);

    // Compose: major lines override minor
    var color = vec4(0.0);
    color = mix(color, in_grid.minor_color, minor_mask * in_grid.minor_color.a);
    color = mix(color, in_grid.major_color, major_mask * in_grid.major_color.a);

    // Apply distance fade
    let dst_to_camera = length(vec3<f32>(in_grid.fade_center.x, 0.0, in_grid.fade_center.y) - vec3<f32>(in.world_xz.x, 0.0, in.world_xz.y));
    let fade_factor = 1.0 - smoothstep(in_grid.fade_start, in_grid.fade_end, dst_to_camera);
    color.a *= fade_factor;

    // Discard if fully transparent (e.g. due to distance fade)
    if color.a < 0.01 { discard; }

    // Output final color
    return color;
}
