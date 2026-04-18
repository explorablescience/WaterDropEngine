use bevy::{
    ecs::system::{SystemParamItem, lifetimeless::SRes},
    prelude::*
};
use wde_renderer::prelude::*;

use crate::prelude::PbrBatchesMarker;

/// Marker component to indicate that an entity's transform should be included in the [PbrSsboTransform] updates.
/// This automatically adds the [PbrBatchesMarker] (and thus the [PbrSsboTransformMarker]) to the entity.
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(PbrBatchesMarker)]
pub struct PbrMaterial3d(pub Handle<PbrMaterial>);

/// Uniform structure for PBR material data.
///
/// The structure sent to the GPU is the following :
/// ```wgsl
/// struct Material3dUniform {
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

/// Describes a physically based rendering material.
/// It contains the material properties such as albedo color, metallic and roughness intensity, as well as the textures for these properties. It also contains a uniform buffer that is automatically created and updated with the material data, which can be used in the shader to access the material properties.
/// The [PbrMaterial3d] component is a wrapper around a handle to a [PbrMaterial] asset, which also adds the [PbrSsboTransformMarker] to the entity, indicating that its transform should be included in the [PbrSsboTransform](crate::prelude::PbrSsboTransform) updates.
#[derive(Asset, Clone, TypePath)]
pub struct PbrMaterial {
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

    /// The buffer containing the material data. It is automatically created and updated if not set.
    pub uniform_buffer: Option<Handle<Buffer>>
}
impl Default for PbrMaterial {
    fn default() -> Self {
        PbrMaterial {
            label: "pbr-material".to_string(),

            albedo: (1.0, 1.0, 1.0, 0.0),
            albedo_t: None,

            metallic: 1.0,
            roughness: 1.0,
            metallic_roughness_t: None,

            normal_t: None,
            occlusion_t: None,

            uniform_buffer: None
        }
    }
}
impl RenderBinding for PbrMaterial {
    type Params = (SRes<AssetServer>, SRes<PlaceholderTexture>);

    fn describe(
        &mut self,
        (asset_server, placeholder_tex): &SystemParamItem<Self::Params>,
        builder: &mut RenderBindingBuilder
    ) {
        // Create the uniform buffer
        let uniform = PbrMaterialUniform {
            flags: [
                if self.albedo_t.is_some() { 1.0 } else { 0.0 },
                if self.metallic_roughness_t.is_some() {
                    1.0
                } else {
                    0.0
                },
                if self.normal_t.is_some() { 1.0 } else { 0.0 },
                if self.occlusion_t.is_some() { 1.0 } else { 0.0 }
            ],
            albedo: [self.albedo.0, self.albedo.1, self.albedo.2, self.albedo.3],
            metallic: self.metallic,
            roughness: self.roughness,
            _padding: [0.0, 0.0]
        };
        if self.uniform_buffer.is_none() {
            let uniform_handle = asset_server.add(Buffer {
                label: format!("{}-uniform-buffer", self.label),
                size: std::mem::size_of::<PbrMaterialUniform>(),
                usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST,
                content: Some(bytemuck::cast_slice(&[uniform]).to_vec())
            });
            self.uniform_buffer = Some(uniform_handle);
        }

        // Build the material
        builder.add_buffer_from_id(Some(self.uniform_buffer.as_ref().unwrap().id()));
        if let Some(albedo_t) = &self.albedo_t {
            builder.add_texture_view_from_id(Some(albedo_t.id()));
            builder.add_texture_sampler_from_id(Some(albedo_t.id()));
        } else {
            builder.add_texture_view_from_id(Some(placeholder_tex.0.id()));
            builder.add_texture_sampler_from_id(Some(placeholder_tex.0.id()));
        }
        if let Some(metallic_roughness_t) = &self.metallic_roughness_t {
            builder.add_texture_view_from_id(Some(metallic_roughness_t.id()));
            builder.add_texture_sampler_from_id(Some(metallic_roughness_t.id()));
        } else {
            builder.add_texture_view_from_id(Some(placeholder_tex.0.id()));
            builder.add_texture_sampler_from_id(Some(placeholder_tex.0.id()));
        }
        if let Some(normal_t) = &self.normal_t {
            builder.add_texture_view_from_id(Some(normal_t.id()));
            builder.add_texture_sampler_from_id(Some(normal_t.id()));
        } else {
            builder.add_texture_view_from_id(Some(placeholder_tex.0.id()));
            builder.add_texture_sampler_from_id(Some(placeholder_tex.0.id()));
        }
        if let Some(occlusion_t) = &self.occlusion_t {
            builder.add_texture_view_from_id(Some(occlusion_t.id()));
            builder.add_texture_sampler_from_id(Some(occlusion_t.id()));
        } else {
            builder.add_texture_view_from_id(Some(placeholder_tex.0.id()));
            builder.add_texture_sampler_from_id(Some(placeholder_tex.0.id()));
        }
    }

    fn label(&self) -> &str {
        &self.label
    }
}
