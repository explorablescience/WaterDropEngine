struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coord:    vec2<f32>,
    @location(1) normal_world: vec3<f32>  // Normal in world space
};

struct FragOutput {
    @location(0) depth: f32,                  // Depth value (NDC, range [0, 1], 1.0 = far plane)
    @location(1) albedo_metallic:  vec4<f32>, // (r, g, b) = albedo color - a = metallic
    @location(2) normal_roughness: vec4<f32>, // (r, g, b) = normal - a = roughness
};

struct PbrMaterialUniform {
    flags: vec4<f32>,    // Flags indicating material textures (1.0 = present, 0.0 = absent) - albedo, metallic-roughness, normal, occlusion
    albedo: vec4<f32>,   // Albedo color of the material (r, g, b)
    metallic: f32,       // Metallic intensity of the material
    roughness: f32,      // Roughness intensity of the material
    _padding: vec2<f32>, // Unused padding to align to 16 bytes
};
@group(2) @binding(0) var<uniform> in_pbr_material: PbrMaterialUniform;    /// Material uniform buffer
@group(2) @binding(1) var in_albedo_texture: texture_2d<f32>;              /// Albedo texture (r, g, b)
@group(2) @binding(2) var in_albedo_sampler: sampler;                      /// Albedo texture sampler
@group(2) @binding(3) var in_metallic_roughness_texture: texture_2d<f32>;  /// Metallic-roughness texture (b = metallic, g = roughness)
@group(2) @binding(4) var in_metallic_roughness_sampler: sampler;          /// Metallic-roughness texture sampler
@group(2) @binding(5) var in_normal_texture: texture_2d<f32>;              /// Normal texture (x, y, z)
@group(2) @binding(6) var in_normal_sampler: sampler;                      /// Normal texture sampler
@group(2) @binding(7) var in_occlusion_texture: texture_2d<f32>;           /// Occlusion texture (r)
@group(2) @binding(8) var in_occlusion_sampler: sampler;                   /// Occlusion texture sampler

@fragment
fn main(in: VertexOutput) -> FragOutput {
    var out: FragOutput;

    // Depth (NDC space: z/w, range [0, 1])
    out.depth = in.clip_position.z / in.clip_position.w;
    
    // Albedo and Metallic
    var albedo_color: vec3<f32> = in_pbr_material.albedo.rgb;
    var metallic_value: f32 = in_pbr_material.metallic;
    if (in_pbr_material.flags.x == 1.0) { // Albedo texture present
        albedo_color = textureSample(in_albedo_texture, in_albedo_sampler, in.tex_coord).rgb;
    }
    if (in_pbr_material.flags.z == 1.0) { // Occlusion texture present
        let occlusion_value: f32 = textureSample(in_occlusion_texture, in_occlusion_sampler, in.tex_coord).r;
        albedo_color *= occlusion_value; // Modulate albedo by occlusion
    }
    if (in_pbr_material.flags.y == 1.0) { // Metallic-roughness texture present
        metallic_value = textureSample(in_metallic_roughness_texture, in_metallic_roughness_sampler, in.tex_coord).b;
    }
    out.albedo_metallic = vec4<f32>(albedo_color, metallic_value);

    // Roughness, and Normal
    var roughness_value: f32 = in_pbr_material.roughness;
    if (in_pbr_material.flags.y == 1.0) { // Metallic-roughness texture present
        roughness_value = textureSample(in_metallic_roughness_texture, in_metallic_roughness_sampler, in.tex_coord).g;
    }
    out.normal_roughness = vec4<f32>(normalize(in.normal_world), roughness_value);
    
    // Note: For now, normal and occlusion textures are not processed in this shader

    // Return G-buffer output
    return out;
}
