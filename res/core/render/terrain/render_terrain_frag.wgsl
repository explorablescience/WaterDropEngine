struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) tex_coord: vec2<f32>,
};

@group(1) @binding(0) var in_heightmap: texture_2d<f32>;
@group(1) @binding(1) var in_heightmap_sampler: sampler;
@group(1) @binding(2) var in_normalmap: texture_2d<f32>;
@group(1) @binding(3) var in_normalmap_sampler: sampler;
@group(1) @binding(4) var in_splatmap_1: texture_2d<f32>;
@group(1) @binding(5) var in_splatmap_1_sampler: sampler;

// Material texture arrays (group 2)
@group(2) @binding(0) var material_albedo: texture_2d_array<f32>;
@group(2) @binding(1) var material_albedo_sampler: sampler;
@group(2) @binding(2) var material_normal: texture_2d_array<f32>;
@group(2) @binding(3) var material_normal_sampler: sampler;
@group(2) @binding(4) var material_roughness: texture_2d_array<f32>;
@group(2) @binding(5) var material_roughness_sampler: sampler;
@group(2) @binding(6) var material_ao: texture_2d_array<f32>;
@group(2) @binding(7) var material_ao_sampler: sampler;

@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Sample the splatmap to determine material blending
    let splatmap = textureSample(in_splatmap_1, in_splatmap_1_sampler, in.tex_coord);
    
    // For now, use the first channel (red) to blend between materials
    // Red channel controls grass (material 0), green for dirt (material 1), etc.
    let weights = vec4<f32>(splatmap.r, splatmap.g, splatmap.b, splatmap.a);
    
    // Sample albedo from each material layer and blend
    var albedo = vec3<f32>(0.0);
    let s = 1.0; // Scale for texture coordinates, adjust as needed
    albedo += textureSample(material_albedo, material_albedo_sampler, in.tex_coord / s, 0).rgb * weights.r;
    albedo += textureSample(material_albedo, material_albedo_sampler, in.tex_coord / s, 1).rgb * weights.g;
    albedo += textureSample(material_albedo, material_albedo_sampler, in.tex_coord / s, 2).rgb * weights.b;
    albedo += textureSample(material_albedo, material_albedo_sampler, in.tex_coord / s, 3).rgb * weights.a;
    
    // Simple lighting calculation
    let sun_dir = normalize(vec3<f32>(0.5, 1.0, 0.5));
    let light = max(dot(in.normal, sun_dir), 0.0);
    
    // Apply simple lighting to albedo
    let lit_color = albedo * (0.3 + 0.7 * light);
    
    return vec4<f32>(lit_color, 1.0);
}
