use wde_renderer::prelude::{Color, *};

use bevy::{
    ecs::system::{SystemParamItem, lifetimeless::SRes},
    prelude::*
};

use crate::prelude::*;

/// The render pass for rendering opaque objects into the G-buffer, in the deferred rendering pipeline.
/// It writes to the depth texture, as well as the albedo, normal and depth textures of the G-buffer, which are then resolved for use in the lighting pass [`crate::prelude::RenderPassDeferredLighting`] (no MSAA in the lighting pass, so we need to resolve the multisampled textures).
///
/// Note:
///  - This render pass is not responsible for rendering transparent objects, which are rendered in a separate render pass[`crate::prelude::RenderPassTransparent`].
///  - This render pass clears the depth texture to 1.0, and the color attachments to black.
///  - It has a render index of 10.
pub struct RenderPassGBuffer;
impl RenderPass for RenderPassGBuffer {
    type Params = (SRes<PbrDeferredTextures>, SBinding<DepthTexture>);

    fn describe(
        (deferred_textures, depth_texture): &SystemParamItem<Self::Params>
    ) -> RenderPassDesc {
        RenderPassDesc {
            attachments_depth: Some(RenderPassDescDepthAttachment {
                texture: depth_texture
                    .iter()
                    .next()
                    .and_then(|(_, t)| t.get_texture(0)),
                load: LoadOp::Clear(1.0),
                ..default()
            }),
            attachments_colors: Some(vec![
                RenderPassDescColorAttachment {
                    texture: deferred_textures.depth.id(),
                    resolve_target: Some(deferred_textures.depth_resolved.id()),
                    load: LoadOp::Clear(Color::BLACK),
                    ..default()
                },
                RenderPassDescColorAttachment {
                    texture: deferred_textures.albedo.id(),
                    resolve_target: Some(deferred_textures.albedo_resolved.id()),
                    load: LoadOp::Clear(Color::BLACK),
                    ..default()
                },
                RenderPassDescColorAttachment {
                    texture: deferred_textures.normal.id(),
                    resolve_target: Some(deferred_textures.normal_resolved.id()),
                    load: LoadOp::Clear(Color::BLACK),
                    ..default()
                },
            ])
        }
    }

    fn id() -> RenderPassId {
        10
    }
    fn label() -> &'static str {
        "deferred-gbuffer"
    }
}
