struct ModelInput {
    @location(0) position:  vec3<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) normal:    vec3<f32>
};
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) tex_coord: vec2<f32>
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

@group(3) @binding(0) var in_heightmap: texture_2d<f32>;
@group(3) @binding(1) var in_heightmap_sampler: sampler;
@group(3) @binding(2) var in_splatmap_1: texture_2d<f32>;
@group(3) @binding(3) var in_splatmap_1_sampler: sampler;


@vertex
fn main(@builtin(instance_index) instance: u32, model: ModelInput) -> VertexOutput {
    var out: VertexOutput;

    // Compute world position
    let tile = in_terrain_tiles[instance];
    let tile_offset = tile.pos * in_terrain_description.tile_size.xz;
    let obj_to_world = mat4x4<f32>(
        vec4<f32>(1.0, 0.0, 0.0, 0.0),
        vec4<f32>(0.0, 1.0, 0.0, 0.0),
        vec4<f32>(0.0, 0.0, 1.0, 0.0),
        vec4<f32>(tile_offset.x, 0.0, tile_offset.y, 1.0)
    );
    var world_pos = obj_to_world * vec4<f32>(model.position, 1.0);

    // Add some noise to the height based on the xz position
    let h = in_terrain_description.tile_size.y;
    world_pos.y = textureSampleLevel(in_heightmap, in_heightmap_sampler, model.tex_coord, 0.0).r * h;

    let view_pos4 = in_camera.world_to_view
        * world_pos;
    let view_pos = view_pos4.xyz / view_pos4.w;
    out.clip_position = in_camera.view_to_ndc * vec4<f32>(view_pos, 1.0);

    // Compute normal with finite differences
    let epsilon = 0.001;
    let height_l = textureSampleLevel(in_heightmap, in_heightmap_sampler, model.tex_coord + vec2<f32>(-epsilon, 0.0), 0.0).r * h;
    let height_r = textureSampleLevel(in_heightmap, in_heightmap_sampler, model.tex_coord + vec2<f32>(epsilon, 0.0), 0.0).r * h;
    let height_d = textureSampleLevel(in_heightmap, in_heightmap_sampler, model.tex_coord + vec2<f32>(0.0, -epsilon), 0.0).r * h;
    let height_u = textureSampleLevel(in_heightmap, in_heightmap_sampler, model.tex_coord + vec2<f32>(0.0, epsilon), 0.0).r * h;
    let normal = normalize(vec3<f32>(height_l - height_r, 2.0 * epsilon, height_d - height_u));
    out.normal = normal;

    // Pass the texture coordinates to the fragment shader
    out.tex_coord = model.tex_coord;

    return out;
}
