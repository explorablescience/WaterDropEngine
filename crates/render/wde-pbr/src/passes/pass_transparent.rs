use bevy::{ecs::system::SystemParamItem, prelude::*};
use wde_renderer::prelude::*;

use crate::prelude::*;

/// The render pass for rendering transparent objects in the deferred rendering pipeline.
/// It uses the depth texture rendered in the previous render pass [`crate::prelude::RenderPassGBuffer`] for depth testing, and writes to the final render texture, which is then presented on the screen.
///
/// Note:
///  - This render pass is responsible for rendering transparent objects, which are rendered in a separate render pass[`crate::prelude::RenderPassDeferredLighting`].
///  - This render pass does not clear the color attachment, as it needs to blend with the opaque objects rendered in the previous render pass. It does not clear the depth attachment either, as it needs to use the depth information rendered in the previous render pass for depth testing.
///  - It has a render index of 50, which means it is executed after the deferred lighting pass, but before the resolve pass. This allows it to blend with the opaque objects rendered in the deferred lighting pass, and be resolved to the swapchain texture in the resolve pass.
pub struct RenderPassTransparent;
impl RenderPass for RenderPassTransparent {
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
                texture: depth_texture.iter().next().map(|(_, t)| t.get_texture(DepthTexture::DEPTH_IDX).unwrap().id()),
                ..Default::default()
            })
        }
    }

    fn id() -> RenderPassId {
        50
    }
    fn label() -> &'static str {
        "transparent"
    }
}
