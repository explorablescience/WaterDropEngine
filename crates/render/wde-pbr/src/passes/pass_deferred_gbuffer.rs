use wde_renderer::prelude::{WdeColor, *};

use bevy::{ecs::system::SystemParamItem, prelude::*};

use crate::prelude::*;

/// The render pass for rendering opaque objects into the G-buffer, in the deferred rendering pipeline.
/// It writes to the depth texture, as well as the depth, albedo, normal and AO textures of the G-buffer, read by the lighting pass [`crate::prelude::RenderPassDeferredLighting`].
///
/// Note:
///  - This render pass is not responsible for rendering transparent objects, which are rendered in a separate render pass[`crate::prelude::RenderPassTransparent`].
///  - This render pass clears the depth texture to 1.0, and the color attachments to black.
///  - It has a render index of 10.
pub struct RenderPassDeferredGBuffer;
impl RenderPass for RenderPassDeferredGBuffer {
    type Params = (SRenderData<DeferredTextures>, SRenderData<DepthTexture>);

    fn describe(
        (deferred_textures, depth_texture): &SystemParamItem<Self::Params>
    ) -> RenderPassDesc {
        let deferred_textures = deferred_textures.iter().next().map(|(_, t)| t).unwrap();
        RenderPassDesc {
            attachments_depth: Some(RenderPassDescDepthAttachment {
                texture: depth_texture
                    .iter()
                    .next()
                    .map(|(_, t)| t.get_texture(DepthTexture::DEPTH_IDX).unwrap().id()),
                load: LoadOp::Clear(1.0),
                stencil_load: LoadOp::Clear(0),
                ..default()
            }),
            attachments_colors: Some(
                [
                    (DeferredTextures::DEPTH_IDX, WdeColor::BLACK),
                    (DeferredTextures::ALBEDO_IDX, WdeColor::BLACK),
                    (DeferredTextures::NORMAL_IDX, WdeColor::BLACK),
                    (DeferredTextures::AO_IDX, WdeColor::WHITE)
                ]
                .into_iter()
                .map(|(idx, clear)| RenderPassDescColorAttachment {
                    texture: deferred_textures.get_texture(idx).map(|t| t.id()),
                    load: LoadOp::Clear(clear),
                    ..default()
                })
                .collect()
            )
        }
    }

    fn id() -> RenderPassId {
        10
    }
    fn label() -> &'static str {
        "deferred-gbuffer"
    }
}
