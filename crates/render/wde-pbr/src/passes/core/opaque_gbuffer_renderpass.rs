use crate::logic::deferred_textures::PbrDeferredTextures;
use bevy::{
    ecs::system::{SystemParamItem, lifetimeless::SRes},
    prelude::*
};
use wde_renderer::prelude::*;

pub struct RenderPassOpaqueGBuffer;
impl RenderPass for RenderPassOpaqueGBuffer {
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
                    load: LoadOp::Clear(WgpuColor {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 1.0
                    }),
                    ..default()
                },
                RenderPassDescColorAttachment {
                    texture: deferred_textures.albedo.id(),
                    resolve_target: Some(deferred_textures.albedo_resolved.id()),
                    load: LoadOp::Clear(WgpuColor {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 1.0
                    }),
                    ..default()
                },
                RenderPassDescColorAttachment {
                    texture: deferred_textures.normal.id(),
                    resolve_target: Some(deferred_textures.normal_resolved.id()),
                    load: LoadOp::Clear(WgpuColor {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 1.0
                    }),
                    ..default()
                },
            ])
        }
    }

    fn id() -> RenderPassId {
        10
    }
    fn label() -> &'static str {
        "opaque-pbr-gbuffer"
    }
}
