use bevy::ecs::system::{SystemParamItem, lifetimeless::SRes};
use wde_renderer::prelude::*;

pub struct RenderPassTerrainGrid;
impl RenderPass for RenderPassTerrainGrid {
    type Params = SRes<DepthTexture>;

    fn describe(depth_texture: &SystemParamItem<Self::Params>) -> RenderPassDesc {
        RenderPassDesc {
            attachments_depth: Some(RenderPassDescDepthAttachment {
                texture: Some(depth_texture.texture.id()),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    fn id() -> RenderPassId { 120 }
    fn label() -> &'static str { "terrain-grid" }
}
