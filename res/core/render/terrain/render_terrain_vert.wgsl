struct ModelInput {
    @location(0) position:  vec3<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) normal:    vec3<f32>
};
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) splatmap_value: vec4<f32>
};

// From world space to normalized device coordinates
struct Camera {
    world_to_view: mat4x4<f32>,
    view_to_ndc: mat4x4<f32>
}
@group(0) @binding(0) var<uniform> in_camera: Camera;

// Description of the terrain
struct TerrainDescription {
    tile_size: vec3<f32>,
    tile_subdivisions: f32,
}
@group(2) @binding(0) var<uniform> in_terrain_description: TerrainDescription;
struct TerrainTile {
    pos: vec2<f32>,
    lod: f32,
    _padding: f32
}
@group(2) @binding(1) var<storage, read> in_terrain_tiles: array<TerrainTile>;

@group(3) @binding(0) var<storage, read> in_heightmap: array<f32>;
@group(3) @binding(1) var<storage, read> in_splatmap_1: array<vec4<f32>>;


@vertex
fn main(@builtin(instance_index) instance: u32, model: ModelInput) -> VertexOutput {
    var out: VertexOutput;

    // Get buffer idx
    let sb = u32(in_terrain_description.tile_subdivisions);
    var tile_idx_x = u32(model.tex_coord.x * in_terrain_description.tile_subdivisions);
    var tile_idx_y = u32(model.tex_coord.y * in_terrain_description.tile_subdivisions);
    let idx = tile_idx_y * sb + tile_idx_x;

    // Compute world position
    let tile_offset = in_terrain_tiles[instance].pos * in_terrain_description.tile_size.xz;
    let obj_to_world = mat4x4<f32>(
        vec4<f32>(1.0, 0.0, 0.0, 0.0),
        vec4<f32>(0.0, 1.0, 0.0, 0.0),
        vec4<f32>(0.0, 0.0, 1.0, 0.0),
        vec4<f32>(tile_offset.x, 0.0, tile_offset.y, 1.0)
    );
    var world_pos = obj_to_world * vec4<f32>(model.position, 1.0);

    // Add some noise to the height based on the xz position
    let h = in_terrain_description.tile_size.y;
    world_pos.y = in_heightmap[idx] * h;

    // Discard if close to edge
    if (tile_idx_x == sb) {
        let idx_left = idx - 1;
        world_pos.y = in_heightmap[idx_left] * h;
    }
    if (tile_idx_y == sb) {
        let idx_up = idx - sb;
        world_pos.y = in_heightmap[idx_up] * h;
    }
    
    // Transform to clip space
    let view_pos4 = in_camera.world_to_view
        * world_pos;
    let view_pos = view_pos4.xyz / view_pos4.w;
    out.clip_position = in_camera.view_to_ndc * vec4<f32>(view_pos, 1.0);

    // Compute normal with finite differences
    let delta = 0.1;
    let idx_right = min(idx + 1, sb * sb - 1);
    let idx_down = min(idx + sb, sb * sb - 1);
    let height_center = in_heightmap[idx] * h;
    let height_right = in_heightmap[idx_right] * h;
    let height_down = in_heightmap[idx_down] * h;
    let normal = normalize(vec3<f32>(height_right - height_center, delta, height_down - height_center));
    out.normal = normal;

    // Pass the texture coordinates to the fragment shader
    out.tex_coord = model.tex_coord;

    // Pass the splatmap value to the fragment shader
    out.splatmap_value = in_splatmap_1[idx];

    return out;
}
