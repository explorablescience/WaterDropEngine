#include "core/render/pbr/pbr_functions.wgsl"
#include "core/render/terrain/terrain_common.wgsl"

struct ModelInput {
    @location(0) position:  vec3<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) normal:    vec3<f32>,
    @location(3) tangent:   vec4<f32>
};
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,  // Clip space position (after projection)
    @location(0) tex_coord:       vec2<f32>,  // Texture coordinates (UV)
    @location(1) normal:          vec3<f32>,  // Normal in local space
    @location(2) normal_world:    vec3<f32>,  // Normal in world space
    @location(3) tangent_world:   vec4<f32>,  // Tangent in world space
    @location(4) bitangent_world: vec3<f32>,  // Bitangent in world space
    @location(5) view_z: f32,                 // View-space Z (linear depth)
    @location(6) tile_pos: vec2<f32>          // Terrain tile index in grid space
};

// From world space to normalized device coordinates
struct Camera {
    world_to_view: mat4x4<f32>,
    view_to_ndc: mat4x4<f32>
}
@group(0) @binding(0) var<uniform> in_camera: Camera;

// Material texture arrays (group 1)
@group(1) @binding(8) var material_displacement: texture_2d_array<f32>;
@group(1) @binding(9) var material_displacement_sampler: sampler;

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

    // Base terrain height from global heightmap
    let h = in_terrain_description.tile_size.y;
    world_pos.y = textureSampleLevel(in_heightmap, in_heightmap_sampler, model.tex_coord, 0.0).r * h - 0.02;

    // Add weighted per-material displacement on top of the base terrain height.
    let splatmap = textureSampleLevel(in_splatmap_1, in_splatmap_1_sampler, model.tex_coord, 0.0);
    let weights = vec4<f32>(splatmap.r, splatmap.g, splatmap.b, splatmap.a);
    let layer_count = terrain_layer_count(textureNumLayers(material_displacement));
    var displacement = 0.0;
    for (var layer: u32 = 0u; layer < layer_count; layer = layer + 1u) {
        let uv = terrain_layer_uv_no_repeat(model.tex_coord, in_terrain_description, tile.pos, layer);
        let w = splat_weight(weights, layer);
        displacement += w * textureSampleLevel(material_displacement, material_displacement_sampler, uv, layer, 0.0).r * in_terrain_description.displacement_scales[layer];
    }
    world_pos.y += displacement;

    // Transform position to clip space
    let view_pos4 = in_camera.world_to_view * world_pos;
    let view_pos = view_pos4.xyz / view_pos4.w;
    out.clip_position = in_camera.view_to_ndc * vec4<f32>(view_pos, 1.0);

    // Store linear view-space depth (negative Z in view space)
    out.view_z = -view_pos.z;

    // Pass through texture coordinates
    out.tex_coord = model.tex_coord;
    out.tile_pos = tile.pos;

    // Compute normal with finite differences from the heightmap
    let epsilon = 0.001;
    let height_l = textureSampleLevel(in_heightmap, in_heightmap_sampler, model.tex_coord + vec2<f32>(-epsilon, 0.0), 0.0).r * h;
    let height_r = textureSampleLevel(in_heightmap, in_heightmap_sampler, model.tex_coord + vec2<f32>(epsilon, 0.0), 0.0).r * h;
    let height_d = textureSampleLevel(in_heightmap, in_heightmap_sampler, model.tex_coord + vec2<f32>(0.0, -epsilon), 0.0).r * h;
    let height_u = textureSampleLevel(in_heightmap, in_heightmap_sampler, model.tex_coord + vec2<f32>(0.0, epsilon), 0.0).r * h;
    let local_normal = normalize(vec3<f32>(height_l - height_r, 2.0 * epsilon, height_d - height_u));
    out.normal = local_normal;

    // Transform normal and tangent to world space
    let nm = normal_matrix(obj_to_world);
    out.normal_world = nm * local_normal;
    out.tangent_world = vec4<f32>(nm * model.tangent.xyz, model.tangent.w);
    out.bitangent_world = cross(out.normal_world, out.tangent_world.xyz) * out.tangent_world.w;

    return out;
}
