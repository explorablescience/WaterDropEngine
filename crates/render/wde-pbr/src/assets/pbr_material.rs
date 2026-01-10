use bevy::prelude::*;
use wde_renderer::prelude::*;

#[derive(Component, Reflect)]
/// Describes a physically based rendering material.
pub struct PbrMaterial(pub Handle<PbrMaterialAsset>);

#[derive(Asset, Clone, TypePath)]
/// Describes a physically based rendering material.
pub struct PbrMaterialAsset {
    /// The label of the material instance.
    pub label: String,

    /// The albedo color of the material instance (r, g, b).
    /// The alpha channel is unused.
    pub albedo: (f32, f32, f32, f32),
    /// The albedo texture of the material instance. If `None`, the material will use the albedo color.
    pub albedo_t: Option<Handle<Texture>>,

    /// The metallic intensity of the material instance.
    pub metallic: f32,
    /// The roughness intensity of the material instance.
    pub roughness: f32,
    /// The metallic-roughness texture of the material instance. If `None`, the material will use the metallic and roughness scalar intensity.
    /// The metallic value is stored in the blue channel, and the roughness value is stored in the green channel.
    pub metallic_roughness_t: Option<Handle<Texture>>,

    /// The normal texture of the material instance.
    /// The normal map is expected to be in tangent space.
    /// The alpha channel is unused.
    pub normal_t: Option<Handle<Texture>>,
    /// The occlusion texture of the material instance.
    /// The occlusion value is stored in the red channel.
    pub occlusion_t: Option<Handle<Texture>>,
}
impl Default for PbrMaterialAsset {
    fn default() -> Self {
        PbrMaterialAsset {
            label: "pbr-material".to_string(),

            albedo:   (1.0, 1.0, 1.0, 0.0),
            albedo_t: None,

            metallic: 1.0,
            roughness: 1.0,
            metallic_roughness_t: None,

            normal_t: None,
            occlusion_t: None,
        }
    }
}

/// Uniform structure for PBR material data.
/// 
/// The structure sent to the GPU is the following :
/// ```wgsl
/// struct PbrMaterialUniform {
///     flags: vec4<f32>, // Flags indicating material textures (1.0 = present, 0.0 = absent) - albedo, metallic-roughness, normal, occlusion
///     albedo: vec4<f32>, // Albedo color of the material (r, g, b)
///     metallic: f32,    // Metallic intensity of the material
///     roughness: f32,   // Roughness intensity of the material
///     _padding: vec2<f32>, // Unused padding to align to 16 bytes
/// };
/// 
/// @group(2) @binding(0) var<uniform> pbr_material: PbrMaterialUniform;    /// Material uniform buffer
/// @group(2) @binding(1) var albedo_texture: texture_2d<f32>;              /// Albedo texture (r, g, b)
/// @group(2) @binding(2) var albedo_sampler: sampler;                      /// Albedo texture sampler
/// @group(2) @binding(3) var metallic_roughness_texture: texture_2d<f32>;  /// Metallic-roughness texture (b = metallic, g = roughness)
/// @group(2) @binding(4) var metallic_roughness_sampler: sampler;          /// Metallic-roughness texture sampler
/// @group(2) @binding(5) var normal_texture: texture_2d<f32>;              /// Normal texture (x, y, z)
/// @group(2) @binding(6) var normal_sampler: sampler;                      /// Normal texture sampler
/// @group(2) @binding(7) var occlusion_texture: texture_2d<f32>;           /// Occlusion texture (r)
/// @group(2) @binding(8) var occlusion_sampler: sampler;                   /// Occlusion texture sampler
/// ```
#[repr(C)]
#[derive(Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct PbrMaterialUniform {
    /// Flags indicating material textures (1.0 = present, 0.0 = absent) - albedo, metallic-roughness, normal, occlusion
    pub flags: [f32; 4],
    /// Albedo color of the material (r, g, b).
    pub albedo: [f32; 4],
    /// Metallic intensity of the material.
    pub metallic: f32,
    /// Roughness intensity of the material.
    pub roughness: f32,
    /// Unused padding to align to 16 bytes.
    _padding: [f32; 2]
}
impl Material for PbrMaterialAsset {
    fn describe(&self, builder: &mut MaterialBuilder) {
        // Create the uniform buffer
        let uniform = PbrMaterialUniform {
            flags: [
                if self.albedo_t.is_some()   { 1.0 } else { 0.0 },
                if self.metallic_roughness_t.is_some() { 1.0 } else { 0.0 },
                if self.normal_t.is_some()   { 1.0 } else { 0.0 },
                if self.occlusion_t.is_some() { 1.0 } else { 0.0 }
            ],
            albedo: [self.albedo.0, self.albedo.1, self.albedo.2, self.albedo.3],
            metallic: self.metallic,
            roughness: self.roughness,
            _padding: [0.0, 0.0],
        };

        // Build the material
        builder.add_buffer(
            0, ShaderStages::FRAGMENT, BufferBindingType::Uniform,
            size_of::<PbrMaterialUniform>(), Some(bytemuck::cast_slice(&[uniform]).to_vec()));
        builder.add_texture_view(    1, ShaderStages::FRAGMENT, self.albedo_t.clone());
        builder.add_texture_sampler( 2, ShaderStages::FRAGMENT, self.albedo_t.clone());
        builder.add_texture_view(    3, ShaderStages::FRAGMENT, self.metallic_roughness_t.clone());
        builder.add_texture_sampler( 4, ShaderStages::FRAGMENT, self.metallic_roughness_t.clone());
        builder.add_texture_view(    5, ShaderStages::FRAGMENT, self.normal_t.clone());
        builder.add_texture_sampler( 6, ShaderStages::FRAGMENT, self.normal_t.clone());
        builder.add_texture_view(    7, ShaderStages::FRAGMENT, self.occlusion_t.clone());
        builder.add_texture_sampler( 8, ShaderStages::FRAGMENT, self.occlusion_t.clone());
    }

    fn label(&self) -> String {
        self.label.to_string() + "-material"
    }
}
