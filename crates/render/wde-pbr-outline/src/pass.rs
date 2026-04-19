use bevy::{ecs::system::SystemParamItem, prelude::*};
use wde_pbr::prelude::{DepthTexture, RenderTexture};
use wde_renderer::prelude::*;

pub struct RenderPassOutline;
impl RenderPass for RenderPassOutline {
    type Params = (SRenderData<DepthTexture>, SRenderData<RenderTexture>);

    fn describe((depth_texture, render_texture): &SystemParamItem<Self::Params>) -> RenderPassDesc {
        RenderPassDesc {
            attachments_colors: Some(vec![RenderPassDescColorAttachment {
                texture: render_texture
                    .iter()
                    .next()
                    .map(|(_, t)| t.get_texture(RenderTexture::BINDING).unwrap().id()),
                ..Default::default()
            }]),
            attachments_depth: Some(RenderPassDescDepthAttachment {
                texture: depth_texture
                    .iter()
                    .next()
                    .map(|(_, t)| t.get_texture(DepthTexture::DEPTH_IDX).unwrap().id()),
                ..Default::default()
            })
        }
    }

    fn id() -> RenderPassId {
        60
    }

    fn label() -> &'static str {
        "pbr-outline"
    }
}
