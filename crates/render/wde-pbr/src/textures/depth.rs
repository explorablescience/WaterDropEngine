use bevy::{ecs::system::{SystemParamItem, lifetimeless::{SQuery, SRes}}, prelude::*};
use wde_renderer::prelude::*;

/// Describes a depth texture that can be used in rendering.
///
/// Notes:
/// - It uses the [`DEPTH_FORMAT`], as well as the [`MSAA_SAMPLE_COUNT`] for multisampling.
/// - It is created and resized automatically based on the window size
#[derive(Asset, Clone, TypePath, Default)]
pub struct DepthTexture(pub Handle<Texture>);
impl DepthTexture {
    pub const DEPTH_IDX: u32 = 0;
}
impl RenderData for DepthTexture {
    type Params = (SQuery<&'static Window>, SRes<Messages<SurfaceResized>>);

    fn describe((window, _): &SystemParamItem<Self::Params>, builder: &mut RenderDataBuilder) {
        let size = {
            let window = window.single().unwrap();
            (
                window.resolution.physical_width(),
                window.resolution.physical_height()
            )
        };
        builder.add_texture(Self::DEPTH_IDX, Texture {
            label: "depth".to_string(),
            size,
            format: DEPTH_FORMAT,
            usages: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            sample_count: MSAA_SAMPLE_COUNT,
            ..Default::default()
        });
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
