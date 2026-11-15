use bevy::{
    ecs::system::{
        SystemParamItem,
        lifetimeless::{SQuery, SRes}
    },
    prelude::*
};
use wde_renderer::prelude::*;

pub(crate) struct DeferredTexturesPlugin;
impl Plugin for DeferredTexturesPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RenderDataRegisterPlugin::<DeferredTextures>::default());
    }
}

pub struct PbrTextureFormat;
impl PbrTextureFormat {
    /// Linear view-space depth. 32 bits, as a 16-bit depth is too coarse to reconstruct positions (shadow banding).
    pub const DEPTH: TextureFormat = TextureFormat::R32Float;
    pub const ALBEDO: TextureFormat = TextureFormat::Rgba8UnormSrgb;
    pub const NORMAL: TextureFormat = TextureFormat::Rgba16Float;
    pub const AO: TextureFormat = TextureFormat::R8Unorm;
}

/// The G-buffer textures, written by the [`RenderPassDeferredGBuffer`](crate::passes::RenderPassDeferredGBuffer).
///
/// They are not multisampled: averaging G-buffer samples at geometry edges mixes the depth, normal and albedo
/// of the object with the background. The depth is not filterable and is read with `textureLoad`.
#[derive(Asset, Clone, TypePath, Default)]
pub struct DeferredTextures;
impl DeferredTextures {
    pub const DEPTH_IDX: u32 = 0;
    pub const ALBEDO_IDX: u32 = 1;
    pub const NORMAL_IDX: u32 = 2;
    pub const AO_IDX: u32 = 3;
}
impl RenderData for DeferredTextures {
    type Params = (SQuery<&'static Window>, SRes<Messages<SurfaceResized>>);

    fn describe((window, _): &mut SystemParamItem<Self::Params>, builder: &mut RenderDataBuilder) {
        let size = {
            let window = window.single().unwrap();
            (
                window.resolution.physical_width(),
                window.resolution.physical_height()
            )
        };
        let usages = TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING;
        for (idx, label, format, filterable) in [
            (Self::DEPTH_IDX, "pbr-depth", PbrTextureFormat::DEPTH, false),
            (
                Self::ALBEDO_IDX,
                "pbr-albedo",
                PbrTextureFormat::ALBEDO,
                true
            ),
            (
                Self::NORMAL_IDX,
                "pbr-normal",
                PbrTextureFormat::NORMAL,
                true
            ),
            (Self::AO_IDX, "pbr-ao", PbrTextureFormat::AO, true)
        ] {
            builder.add_texture(
                idx,
                Texture {
                    label: label.to_string(),
                    size,
                    format,
                    usages,
                    filterable,
                    ..Default::default()
                }
            );
        }
    }

    fn recreate((_, surface_resized): &SystemParamItem<Self::Params>) -> Option<bool> {
        Some(
            surface_resized
                .get_cursor()
                .read(surface_resized)
                .next()
                .is_some()
        )
    }
}
