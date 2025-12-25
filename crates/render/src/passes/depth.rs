use bevy::prelude::*;
use wde_wgpu::{bind_group::{BindGroup, BindGroupLayout, BindGroupLayoutBuilder, WgpuBindGroup}, instance::WRenderInstance, render_pipeline::WShaderStages, texture::{WTexture, WTextureUsages}};

use crate::{assets::{GpuTexture, RenderAssets, Texture}, core::{extract_macros::ExtractWorld, window::SurfaceResized}};

#[derive(Resource, Default)]
pub struct DepthTextureLayoutRegenerate(pub bool);

#[derive(Resource, Default)]
pub struct DepthTextureLayout {
    pub layout: Option<BindGroupLayout>,
    pub bind_group: Option<WgpuBindGroup>
}
impl DepthTextureLayout {
    pub fn build_bind_group(
        render_instance: Res<WRenderInstance<'static>>, mut textures_layout: ResMut<DepthTextureLayout>,
        depth_texture: Res<DepthTexture>, textures: Res<RenderAssets<GpuTexture>>
    ) {
        // Check if the bind group is already created
        if textures_layout.bind_group.is_some() & textures_layout.layout.is_some() {
            return;
        }

        // Get the depth texture
        let depth_texture = match textures.get(&depth_texture.texture) {
            Some(texture) => texture,
            None => return
        };

        // Create the deferred layout
        let layout = BindGroupLayout::new("depth-texture", |builder: &mut BindGroupLayoutBuilder| {
            builder.add_depth_texture_view(   0, WShaderStages::FRAGMENT);
            builder.add_depth_texture_sampler(1, WShaderStages::FRAGMENT);
        });

        // Build the layout
        let render_instance = render_instance.data.read().unwrap();
        let layout_built = BindGroupLayout::build(&layout, &render_instance);

        // Create the bind group
        let bind_group = BindGroup::build("depth-texture", &render_instance, &layout_built, &vec![
            BindGroup::texture_view(   0, &depth_texture.texture),
            BindGroup::texture_sampler(1, &depth_texture.texture)
        ]);

        // Insert the resources
        textures_layout.layout = Some(layout);
        textures_layout.bind_group = Some(bind_group);
    }
}


#[derive(Resource)]
pub struct DepthTexture {
    pub texture: Handle<Texture>,
    pub resized: bool
}
impl DepthTexture {
    pub fn create_texture(mut commands: Commands, server: Res<AssetServer>, window: Query<&Window>) {
        let resolution = &window.single().unwrap().resolution;
        let texture = server.add(Texture {
            label: "depth".to_string(),
            size: (resolution.physical_width(), resolution.physical_height()),
            format: WTexture::DEPTH_FORMAT,
            usages: WTextureUsages::RENDER_ATTACHMENT | WTextureUsages::TEXTURE_BINDING,
            ..Default::default()
        });
        commands.insert_resource(DepthTexture { texture, resized: false });
    }

    pub fn resize_texture(
        mut window_resized_events: EventReader<SurfaceResized>,
        server: Res<AssetServer>, mut textures: ResMut<DepthTexture>
    ) {
        textures.resized = false;
        for event in window_resized_events.read() {
            // Recreate the depth texture
            let texture = server.add(Texture {
                label: "depth".to_string(),
                size: (event.width, event.height),
                format: WTexture::DEPTH_FORMAT,
                usages: WTextureUsages::RENDER_ATTACHMENT | WTextureUsages::TEXTURE_BINDING,
                ..Default::default()
            });

            // Insert the resources
            textures.texture = texture;
            textures.resized = true;
        }
    }

    pub fn extract_texture(mut commands: Commands, depth_texture : ExtractWorld<Res<DepthTexture>>, mut depth_texture_layout: ResMut<DepthTextureLayout>) {
        if depth_texture.resized {
            depth_texture_layout.layout = None;
            depth_texture_layout.bind_group = None;
        }

        commands.insert_resource(DepthTexture {
            texture: depth_texture.texture.clone(),
            resized: false
        });
    }
}
