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

#[derive(Asset, Clone, TypePath, Default)]
pub struct DeferredTextures;
impl DeferredTextures {
    pub const DEPTH_IDX: u32 = 0;
    pub const DEPTH_RESOLVED_IDX: u32 = 1;
    pub const ALBEDO_IDX: u32 = 2;
    pub const ALBEDO_RESOLVED_IDX: u32 = 3;
    pub const NORMAL_IDX: u32 = 4;
    pub const NORMAL_RESOLVED_IDX: u32 = 5;
    pub const AO_IDX: u32 = 6;
    pub const AO_RESOLVED_IDX: u32 = 7;
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
        builder
            .add_texture(
                DeferredTextures::DEPTH_IDX,
                Texture {
                    label: "pbr-depth".to_string(),
                    size,
                    format: TextureFormat::R16Float,
                    usages,
                    sample_count: MSAA_SAMPLE_COUNT,
                    ..Default::default()
                }
            )
            .add_texture(
                DeferredTextures::DEPTH_RESOLVED_IDX,
                Texture {
                    label: "pbr-depth-resolved".to_string(),
                    size,
                    format: TextureFormat::R16Float,
                    usages,
                    ..Default::default()
                }
            )
            .add_texture(
                DeferredTextures::ALBEDO_IDX,
                Texture {
                    label: "pbr-albedo".to_string(),
                    size,
                    format: TextureFormat::Rgba8UnormSrgb,
                    usages,
                    sample_count: MSAA_SAMPLE_COUNT,
                    ..Default::default()
                }
            )
            .add_texture(
                DeferredTextures::ALBEDO_RESOLVED_IDX,
                Texture {
                    label: "pbr-albedo-resolved".to_string(),
                    size,
                    format: TextureFormat::Rgba8UnormSrgb,
                    usages,
                    ..Default::default()
                }
            )
            .add_texture(
                DeferredTextures::NORMAL_IDX,
                Texture {
                    label: "pbr-normal".to_string(),
                    size,
                    format: TextureFormat::Rgba16Float,
                    usages,
                    sample_count: MSAA_SAMPLE_COUNT,
                    ..Default::default()
                }
            )
            .add_texture(
                DeferredTextures::NORMAL_RESOLVED_IDX,
                Texture {
                    label: "pbr-normal-resolved".to_string(),
                    size,
                    format: TextureFormat::Rgba16Float,
                    usages,
                    ..Default::default()
                }
            )
            .add_texture(
                DeferredTextures::AO_IDX,
                Texture {
                    label: "pbr-ao".to_string(),
                    size,
                    format: TextureFormat::R8Unorm,
                    usages,
                    sample_count: MSAA_SAMPLE_COUNT,
                    ..Default::default()
                }
            )
            .add_texture(
                DeferredTextures::AO_RESOLVED_IDX,
                Texture {
                    label: "pbr-ao-resolved".to_string(),
                    size,
                    format: TextureFormat::R8Unorm,
                    usages,
                    ..Default::default()
                }
            );
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
