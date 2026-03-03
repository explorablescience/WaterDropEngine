struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) splatmap_value: vec4<f32>
};

// Material texture arrays (group 2)
@group(1) @binding(0) var material_albedo: texture_2d_array<f32>;
@group(1) @binding(1) var material_albedo_sampler: sampler;
@group(1) @binding(2) var material_normal: texture_2d_array<f32>;
@group(1) @binding(3) var material_normal_sampler: sampler;
@group(1) @binding(4) var material_roughness: texture_2d_array<f32>;
@group(1) @binding(5) var material_roughness_sampler: sampler;
@group(1) @binding(6) var material_ao: texture_2d_array<f32>;
@group(1) @binding(7) var material_ao_sampler: sampler;

// Description of the terrain
struct TerrainDescription {
    tile_size: vec3<f32>,
    tile_subdivisions: f32,
}
@group(2) @binding(0) var<uniform> in_terrain_description: TerrainDescription;

// Terrain tile data
@group(3) @binding(0) var<storage, read> in_heightmap: array<f32>;
@group(3) @binding(1) var<storage, read> in_splatmap_1: array<vec4<f32>>;

@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    let idx = u32(in.tex_coord.y * in_terrain_description.tile_subdivisions) * u32(in_terrain_description.tile_subdivisions) + u32(in.tex_coord.x * in_terrain_description.tile_subdivisions);

    // Sample the splatmap to determine material blending
    let weights = in.splatmap_value; // RGBA channels correspond to 4 different materials

    // Compute material UVs (could be world position based for better tiling)
    let material_uv = in.tex_coord * 25.0 % 1.0; // Simple tiling based on vertex UVs
    
    // Sample albedo from each material layer and blend
    var albedo = vec3<f32>(0.0);
    albedo += textureSample(material_albedo, material_albedo_sampler, material_uv, 0).rgb * weights.x;
    albedo += textureSample(material_albedo, material_albedo_sampler, material_uv, 1).rgb * weights.y;
    albedo += textureSample(material_albedo, material_albedo_sampler, material_uv, 2).rgb * weights.z;
    albedo += textureSample(material_albedo, material_albedo_sampler, material_uv, 3).rgb * weights.w;
    
    // Simple lighting calculation
    let sun_dir = normalize(vec3<f32>(0.5, 1.0, 0.5));
    let light = max(dot(in.normal, sun_dir), 0.0);
    
    // Apply simple lighting to albedo
    let lit_color = albedo * (0.3 + 0.7 * light);
    
    return vec4<f32>(lit_color, 1.0);
}
