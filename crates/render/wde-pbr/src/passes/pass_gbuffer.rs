use wde_renderer::prelude::{Color, *};

use bevy::{ecs::system::SystemParamItem, prelude::*};

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
    type Params = (
        SBinding<DeferredTextures>,
        SBinding<DeferredTexturesResolved>,
        SBinding<DepthTexture>
    );

    fn describe(
        (deferred_textures, deferred_textures_resolved, depth_texture): &SystemParamItem<
            Self::Params
        >
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
                    texture: deferred_textures
                        .iter()
                        .next()
                        .map(|(_, t)| t.get_texture(DeferredTextures::DEPTH_BINDING).unwrap())
                        .unwrap(),
                    resolve_target: deferred_textures_resolved.iter().next().map(|(_, t)| {
                        t.get_texture(DeferredTexturesResolved::DEPTH_BINDING)
                            .unwrap()
                    }),
                    load: LoadOp::Clear(Color::BLACK),
                    ..default()
                },
                RenderPassDescColorAttachment {
                    texture: deferred_textures
                        .iter()
                        .next()
                        .map(|(_, t)| t.get_texture(DeferredTextures::ALBEDO_BINDING).unwrap())
                        .unwrap(),
                    resolve_target: deferred_textures_resolved.iter().next().map(|(_, t)| {
                        t.get_texture(DeferredTexturesResolved::ALBEDO_BINDING)
                            .unwrap()
                    }),
                    load: LoadOp::Clear(Color::BLACK),
                    ..default()
                },
                RenderPassDescColorAttachment {
                    texture: deferred_textures
                        .iter()
                        .next()
                        .map(|(_, t)| t.get_texture(DeferredTextures::NORMAL_BINDING).unwrap())
                        .unwrap(),
                    resolve_target: deferred_textures_resolved.iter().next().map(|(_, t)| {
                        t.get_texture(DeferredTexturesResolved::NORMAL_BINDING)
                            .unwrap()
                    }),
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
