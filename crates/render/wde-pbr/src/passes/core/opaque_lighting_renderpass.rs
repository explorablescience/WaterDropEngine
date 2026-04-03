use bevy::ecs::system::{SystemParamItem, lifetimeless::SRes};
use wde_renderer::prelude::*;

use crate::logic::render_texture::PbrRenderTexture;


pub struct RenderPassOpaqueLighting;
impl RenderPass for RenderPassOpaqueLighting {
    type Params = SRes<PbrRenderTexture>;

    fn describe(render_texture: &SystemParamItem<Self::Params>) -> RenderPassDesc {
        RenderPassDesc {
            attachments_colors: Some(vec![
                RenderPassDescColorAttachment {
                    texture: render_texture.texture.id(),
                    ..Default::default()
                },
            ]),
            ..Default::default()
        }
    }

    fn id() -> RenderPassId { 20 }
    fn label() -> &'static str { "opaque-pbr-lighting" }
}
