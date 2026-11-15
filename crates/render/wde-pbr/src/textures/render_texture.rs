use bevy::{
    ecs::system::{
        SystemParamItem,
        lifetimeless::{SQuery, SRes}
    },
    prelude::*
};
use wde_renderer::prelude::*;

pub(crate) struct RenderTexturePlugin;
impl Plugin for RenderTexturePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RenderDataRegisterPlugin::<RenderTexture>::default());
    }
}

/// The HDR render texture of the pbr renderer, in linear color.
///
/// Notes:
/// - It uses the [`RenderTexture::FORMAT`], without multisampling (anti-aliasing is done in post-processing). Pipelines drawing into it must declare this format in their `render_targets`.
/// - It is created and resized automatically based on the window size
/// - It is resolved, tonemapped and post-processed to the swapchain at the end of the frame (see [`crate::post_process`]).
#[derive(Asset, Clone, TypePath, Default)]
pub struct RenderTexture;
impl RenderTexture {
    pub const BINDING: u32 = 0;
    pub const FORMAT: TextureFormat = TextureFormat::Rgba16Float;
}
impl RenderData for RenderTexture {
    type Params = (SQuery<&'static Window>, SRes<Messages<SurfaceResized>>);

    fn describe((window, _): &mut SystemParamItem<Self::Params>, builder: &mut RenderDataBuilder) {
        let size = {
            let window = window.single().unwrap();
            (
                window.resolution.physical_width(),
                window.resolution.physical_height()
            )
        };
        builder.add_texture(
            RenderTexture::BINDING,
            Texture {
                label: "pbr-render-texture".to_string(),
                size,
                format: RenderTexture::FORMAT,
                usages: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
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
