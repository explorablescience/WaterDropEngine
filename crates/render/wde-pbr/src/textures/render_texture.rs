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
        app.add_plugins((
            RenderDataRegisterPlugin::<RenderTexture>::default(),
            RenderBindingRegisterPlugin::<RenderTextureBinding>::default()
        ));
    }
}

/// The render texture for the pbr renderer.
///
/// Notes:
/// - It uses the [`SWAPCHAIN_FORMAT`], as well as the [`MSAA_SAMPLE_COUNT`] for multisampling.
/// - It is created and resized automatically based on the window size
/// - It is extracted and made available in the render world as a bind group for the lighting pass, so it can be sampled as an input texture.
/// - It is rendered to the swapchain in the final pass.
#[derive(Asset, Clone, TypePath, Default)]
pub struct RenderTexture;
impl RenderTexture {
    pub const BINDING: u32 = 0;
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
                format: SWAPCHAIN_FORMAT,
                usages: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                sample_count: MSAA_SAMPLE_COUNT,
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

/// The render binding for the render texture, which makes it available as a bind group in the render world.
#[derive(Asset, Clone, TypePath, Default)]
pub(crate) struct RenderTextureBinding;
impl RenderBinding for RenderTextureBinding {
    type Params = SRenderData<RenderTexture>;

    fn describe(
        &mut self,
        tex: &SystemParamItem<Self::Params>,
        builder: &mut RenderBindingBuilder
    ) {
        builder.add_texture_view(tex, RenderTexture::BINDING);
        builder.add_texture_sampler(tex, RenderTexture::BINDING);
    }

    fn label(&self) -> &str {
        "render_texture_binding"
    }
}
