//! Bind groups of the [`UiImage`](crate::node::UiImage) textures, and the draw calls using them.
use std::ops::Range;

use bevy::{platform::collections::HashMap, prelude::*};
use wde_logger::prelude::*;
use wde_renderer::prelude::*;
use wde_wgpu::{
    bind_group::{BindGroup, BindGroupBuilder, BindGroupLayout, WgpuBindGroupLayout},
    instance::RenderInstanceData
};

use crate::render::data::UiQuad;

/// Layout of the image bind group (group 1): a texture and its sampler.
#[derive(Resource, Clone)]
pub(crate) struct UiImageLayout(pub BindGroupLayout);
impl Default for UiImageLayout {
    fn default() -> Self {
        Self(BindGroupLayout::new("ui-image", |builder| {
            builder
                .add_texture_view(0, ShaderStages::FRAGMENT, false)
                .add_texture_sampler(1, ShaderStages::FRAGMENT);
        }))
    }
}

/// Quads of this frame, as draw calls sharing the same image bind group.
#[derive(Resource, Default)]
pub(crate) struct UiDrawBatches(pub Vec<(BindGroup, Range<u32>)>);

/// Bind groups of the image textures, built on first use.
#[derive(Resource, Default)]
pub(crate) struct UiImageBindGroups {
    layout: Option<WgpuBindGroupLayout>,
    groups: HashMap<AssetId<Texture>, BindGroup>
}
impl UiImageBindGroups {
    pub(crate) fn clear(&mut self) {
        self.groups.clear();
    }

    /// Splits the quads into consecutive runs using the same image, keeping their draw order.
    /// Quads without an image join any run and are drawn with `placeholder` bound. Quads whose
    /// image is still loading are hidden.
    pub(crate) fn batch(
        &mut self,
        instance: &RenderInstanceData,
        textures: &RenderAssets<GpuTexture>,
        placeholder: Option<AssetId<Texture>>,
        quads: &mut [UiQuad],
        images: &[Option<AssetId<Texture>>]
    ) -> Vec<(BindGroup, Range<u32>)> {
        let mut runs: Vec<(Option<AssetId<Texture>>, Range<u32>)> = Vec::new();
        for (i, (quad, image)) in quads.iter_mut().zip(images).enumerate() {
            let image = image.filter(|id| textures.get(*id).is_some());
            if images[i].is_some() && image.is_none() {
                quad.color[3] = 0.0;
                quad.radius[1] = 0.0;
            }
            let i = i as u32;
            match runs.last_mut() {
                Some((run_image, range))
                    if image.is_none() || run_image.is_none_or(|r| Some(r) == image) =>
                {
                    *run_image = run_image.or(image);
                    range.end = i + 1;
                }
                _ => runs.push((image, i..i + 1))
            }
        }

        runs.into_iter()
            .filter_map(|(image, range)| {
                let bind_group = self.bind_group(instance, textures, image.or(placeholder)?)?;
                Some((bind_group, range))
            })
            .collect()
    }

    fn bind_group(
        &mut self,
        instance: &RenderInstanceData,
        textures: &RenderAssets<GpuTexture>,
        image: AssetId<Texture>
    ) -> Option<BindGroup> {
        if let Some(bind_group) = self.groups.get(&image) {
            return Some(bind_group.clone());
        }
        let texture = &textures.get(image)?.texture;
        if self.layout.is_none() {
            self.layout = UiImageLayout::default().0.build(instance).ok();
        }
        let bind_group = BindGroupBuilder::build(
            "ui-image",
            instance,
            self.layout.as_ref()?,
            &vec![
                BindGroupBuilder::texture_view(0, texture),
                BindGroupBuilder::texture_sampler(1, texture),
            ]
        );
        match bind_group {
            Ok(bind_group) => {
                self.groups.insert(image, bind_group.clone());
                Some(bind_group)
            }
            Err(e) => {
                error!("Failed to create the UI image bind group: {:?}.", e);
                None
            }
        }
    }
}
