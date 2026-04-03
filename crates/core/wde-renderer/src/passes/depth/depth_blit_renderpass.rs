use bevy::ecs::system::{SystemParamItem, lifetimeless::SRes};

use crate::prelude::*;

pub struct RenderPassDepthBlit;
impl RenderPass for RenderPassDepthBlit {
    type Params = SRes<DepthTexture>;

    fn describe(depth_texture: &SystemParamItem<Self::Params>) -> RenderPassDesc {
        RenderPassDesc {
            attachments_colors: Some(vec![]),
            attachments_depth: Some(RenderPassDescDepthAttachment {
                texture: Some(depth_texture.texture.id()),
                load: LoadOp::Load,
                ..Default::default()
            })
        }
    }

    fn id() -> RenderPassId { 100 }
    fn label() -> &'static str { "depth-blit" }
}
