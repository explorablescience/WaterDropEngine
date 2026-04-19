use wde_pbr::deferred::PbrMaterial;
use wde_renderer::prelude::{Color, *};

use bevy::{
    ecs::system::{SystemParamItem, lifetimeless::SRes},
    prelude::*
};

#[repr(C)]
#[derive(Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct GhostMaterialUniform {
    pub overlay: [f32; 4],
    pub albedo: [f32; 4],
    pub albedo_texture_present: f32,
    pub desaturate_factor: f32,
    pub rim_power: f32,
    pub rim_strength: f32
}

#[derive(Asset, Clone, TypePath)]
pub struct GhostMaterial {
    pub overlay: Color,
    pub desaturate: f32,
    pub rim_power: f32,
    pub rim_strength: f32,
    pub pbr_material: Option<Handle<PbrMaterial>>,

    pub buffer: Option<Handle<Buffer>>
}
impl Default for GhostMaterial {
    fn default() -> Self {
        Self {
            overlay: Color::WHITE,
            desaturate: 0.5,
            rim_power: 5.0,
            rim_strength: 0.1,
            pbr_material: None,
            buffer: None
        }
    }
}
impl RenderBinding for GhostMaterial {
    type Params = (
        SRes<AssetServer>,
        SBinding<PbrMaterial>,
        SRes<PlaceholderTexture>
    );

    fn describe(
        &mut self,
        (asset_server, render_assets, placeholder_texture): &SystemParamItem<Self::Params>,
        builder: &mut RenderBindingBuilder
    ) {
        // Dummy material for initialization
        if self.pbr_material.is_none() {
            if self.buffer.is_none() {
                self.buffer = Some(asset_server.add(Buffer {
                    label: "ghost-material".to_string(),
                    size: std::mem::size_of::<GhostMaterialUniform>(),
                    usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST,
                    content: Some(
                        bytemuck::cast_slice(&[GhostMaterialUniform::default()]).to_vec()
                    )
                }));
            }
            builder.add_buffer_from_id(Some(self.buffer.as_ref().unwrap().id()));
            builder.add_texture_view_from_id(Some(placeholder_texture.0.id()));
            builder.add_texture_sampler_from_id(Some(placeholder_texture.0.id()));
            return;
        }

        // Get PBR material
        let (albedo, albedo_t, is_albedo_present) =
            match render_assets.get(self.pbr_material.as_ref().unwrap()) {
                Some(material) => match material.asset.albedo_t.as_ref() {
                    Some(t) => (material.asset.albedo, t.id(), true),
                    None => (material.asset.albedo, placeholder_texture.0.id(), false)
                },
                None => {
                    builder.retry_next_update();
                    return;
                }
            };

        // Create the uniform buffer
        if self.buffer.is_none() {
            let uniform = GhostMaterialUniform {
                overlay: [
                    self.overlay.r(),
                    self.overlay.g(),
                    self.overlay.b(),
                    self.overlay.a()
                ],
                albedo: [albedo.0, albedo.1, albedo.2, albedo.3],
                albedo_texture_present: if is_albedo_present { 1.0 } else { 0.0 },
                desaturate_factor: self.desaturate,
                rim_power: self.rim_power,
                rim_strength: self.rim_strength
            };
            let buffer = asset_server.add(Buffer {
                label: "ghost-material".to_string(),
                size: std::mem::size_of::<GhostMaterialUniform>(),
                usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST,
                content: Some(bytemuck::cast_slice(&[uniform]).to_vec())
            });
            self.buffer = Some(buffer);
        }
        builder.add_buffer_from_id(Some(self.buffer.as_ref().unwrap().id()));

        // Get PBR material
        builder.add_texture_view_from_id(Some(albedo_t));
        builder.add_texture_sampler_from_id(Some(albedo_t));
    }

    fn label(&self) -> &str {
        "ghost-material"
    }
}
