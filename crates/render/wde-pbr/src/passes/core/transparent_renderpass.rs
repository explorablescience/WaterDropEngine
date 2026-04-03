use bevy::{ecs::system::{SystemParamItem, lifetimeless::SRes}, prelude::*};
use wde_renderer::prelude::*;

use crate::logic::render_texture::PbrRenderTexture;


pub struct TransparentRenderPass;
impl RenderPass for TransparentRenderPass {
    type Params = (SRes<DepthTextureMSAA>, SRes<PbrRenderTexture>);

    fn describe((depth_texture, render_texture): &SystemParamItem<Self::Params>) -> RenderPassDesc {
        RenderPassDesc {
            attachments_colors: Some(vec![
                RenderPassDescColorAttachment {
                    texture: render_texture.texture.id(),
                    ..Default::default()
                },
            ]),
            attachments_depth: Some(RenderPassDescDepthAttachment {
                texture: Some(depth_texture.texture.id()),
                ..default()
            })
        }
    }

    fn id() -> RenderPassId { 40 }
    fn label() -> &'static str { "transparent-pbr" }
}
