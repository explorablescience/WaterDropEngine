struct ModelInput {
    @location(0) position:  vec3<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) normal:    vec3<f32>,
    @location(3) tangent:   vec4<f32>
};
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_xz: vec2<f32>, // world-space XZ of fragment
    @location(1) chunk_uv: vec2<f32>  // [0,1] within the matched chunk
};

// From world space to normalized device coordinates
struct Camera {
    world_to_view: mat4x4<f32>,
    view_to_ndc: mat4x4<f32>
}
@group(0) @binding(0) var<uniform> in_camera: Camera;

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

@vertex
fn main(@builtin(instance_index) instance: u32, model: ModelInput) -> VertexOutput {
    var out: VertexOutput;

    let elevation = model.position.z + 0.01; // Avoid z-fighting

    // Compute world position
    let local = model.position.xy * in_grid.chunk_size;
    let world = local + in_chunk_positions[instance].xz;
    let world_pos_3 = vec3<f32>(world.x, elevation, world.y);

    // Set outputs
    out.clip_position = in_camera.view_to_ndc * in_camera.world_to_view * vec4<f32>(world_pos_3, 1.0);
    out.world_xz = world_pos_3.xz;
    out.chunk_uv = model.tex_coord;
    
    return out;
}
