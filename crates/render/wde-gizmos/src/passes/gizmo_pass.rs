use bevy::ecs::system::{SystemParamItem, lifetimeless::SRes};
use wde_renderer::prelude::*;

pub struct GizmoRenderPass;
impl RenderPass for GizmoRenderPass {
    // type Params = SRes<DepthTexture>;
    type Params = ();

    fn describe(depth_texture: &SystemParamItem<Self::Params>) -> RenderPassDesc {
        RenderPassDesc {
            attachments_depth: Some(RenderPassDescDepthAttachment {
                // texture: Some(depth_texture.texture.id()),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    fn id() -> RenderPassId { 500 }
    fn label() -> &'static str { "gizmo" }
}
