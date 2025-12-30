use bevy::prelude::*;
use wde_wgpu::{bind_group::WBufferBindingType, render_pipeline::WShaderStages};

use crate::assets::{Material, MaterialBuilder, Texture};

#[derive(Asset, Clone, TypePath)]
/// Describes a physically based rendering material.
pub struct PbrMaterialAsset {
    /// The label of the material instance.
    pub label: String,

    /// The albedo color of the material instance.
    pub albedo: (f32, f32, f32, f32),
    /// The albedo texture of the material instance. If `None`, the material will use the albedo color.
    pub albedo_t: Option<Handle<Texture>>,

    /// The specular intensity of the material instance.
    pub specular: f32,
    /// The specular texture of the material instance. If `None`, the material will use the specular intensity.
    pub specular_t: Option<Handle<Texture>>,
}
impl Default for PbrMaterialAsset {
    fn default() -> Self {
        PbrMaterialAsset {
            label: "pbr-material".to_string(),

            albedo:   (1.0, 1.0, 1.0, 1.0),
            albedo_t: None,

            specular:   1.0,
            specular_t: None,
        }
    }
}
#[derive(Component, Reflect)]
/// Describes a physically based rendering material.
pub struct PbrMaterial(pub Handle<PbrMaterialAsset>);


#[repr(C)]
#[derive(Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct PbrMaterialUniform {
    /// Flags indicating material textures.
    pub flags: [f32; 4],
    /// RGB albedo of the material.
    pub albedo: [f32; 4],
    /// Specular intensity of the material.
    pub specular: f32,
    /// Unused padding.
    _padding: [f32; 3]
}
impl Material for PbrMaterialAsset {
    fn describe(&self, builder: &mut MaterialBuilder) {
        // Create the uniform buffer
        let uniform = PbrMaterialUniform {
            flags: [
                if self.albedo_t.is_some()   { 1.0 } else { 0.0 },
                if self.specular_t.is_some() { 1.0 } else { 0.0 },
                0.0, // Unused
                0.0, // Unused
            ],
            albedo: [self.albedo.0, self.albedo.1, self.albedo.2, self.albedo.3],
            specular: self.specular,
            _padding: [0.0; 3],
        };

        // Build the material
        builder.add_buffer(
            0, WShaderStages::FRAGMENT, WBufferBindingType::Uniform,
            size_of::<PbrMaterialUniform>(), Some(bytemuck::cast_slice(&[uniform]).to_vec()));
        builder.add_texture_view(    1, WShaderStages::FRAGMENT, self.albedo_t.clone());
        builder.add_texture_sampler( 2, WShaderStages::FRAGMENT, self.albedo_t.clone());
        builder.add_texture_view(    3, WShaderStages::FRAGMENT, self.specular_t.clone());
        builder.add_texture_sampler( 4, WShaderStages::FRAGMENT, self.specular_t.clone());
    }

    fn label(&self) -> String {
        self.label.to_string() + "-material"
    }
}
