//! Shared depth texture lifecycle and bind group binding.
//!
//! The depth texture is created at startup, resized on window events, extracted to the render
//! world, and made available as a bind group in the render phase. Most passes can bind this
//! depth resource for sampling in fragment shaders or depth testing.

use bevy::prelude::*;
use wde_wgpu::{bind_group::{BindGroupBuilder, BindGroupLayout, BindGroupLayoutBuilder, WgpuBindGroup}, render_pipeline::ShaderStages, texture::{DEPTH_FORMAT, TextureUsages}};

use crate::{MSAA_SAMPLE_COUNT, core::RenderInstance};

use crate::{assets::{GpuTexture, RenderAssets, Texture}, core::{extract_macros::ExtractWorld, window::SurfaceResized}};


/// Stores the built bind group layout and group for depth sampling.
///
/// The layout has two bindings:
/// - **Binding 0**: Depth texture view (read-only)
/// - **Binding 1**: Depth sampler (for filtered sampling)
#[derive(Resource, Default)]
pub struct DepthMSAATextureLayout {
    /// Bind group layout describing depth bindings (if built).
    pub layout: Option<BindGroupLayout>,
    /// Actual bind group with depth texture/sampler (if built).
    pub bind_group: Option<WgpuBindGroup>
}
impl DepthMSAATextureLayout {
    /// Build the depth bind group and layout (or skip if already built and valid).
    ///
    /// Called during the Render phase once GPU textures are ready.
    /// Rebuilds if depth was resized; otherwise reuses cached bind group.
    pub fn build_bind_group(
        render_instance: Res<RenderInstance>, mut textures_layout: ResMut<DepthMSAATextureLayout>,
        depth_texture: Res<DepthTextureMSAA>, textures: Res<RenderAssets<GpuTexture>>
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

        // Create the layout
        let layout = BindGroupLayout::new("depth-msaa-texture", |builder: &mut BindGroupLayoutBuilder| {
            builder.add_texture_view(   0, ShaderStages::FRAGMENT, true);
            builder.add_texture_sampler(1, ShaderStages::FRAGMENT);
        });

        // Build the layout
        let render_instance = render_instance.0.read().unwrap();
        let layout_built = BindGroupLayout::build(&layout, &render_instance);

        // Create the bind group
        let bind_group = BindGroupBuilder::build("depth-msaa-texture", &render_instance, &layout_built, &vec![
            BindGroupBuilder::texture_view(   0, &depth_texture.texture),
            BindGroupBuilder::texture_sampler(1, &depth_texture.texture)
        ]);

        // Insert the resources
        textures_layout.layout = Some(layout);
        textures_layout.bind_group = Some(bind_group);
    }
}


/// Holds the handle to the depth texture asset in the main and render worlds.
///
/// The texture is created in Startup, resized on window events, and extracted to the
/// render world during the Extract schedule. The handle references a [`Texture`] asset
/// that is uploaded to GPU asynchronously by the render assets pipeline.
#[derive(Resource)]
pub struct DepthTextureMSAA {
    /// Asset handle pointing to the depth texture (CPU/GPU representation).
    pub texture: Handle<Texture>,
    /// Flag set to true when the texture was resized this frame.
    pub resized: bool
}
impl DepthTextureMSAA {
    /// Create the initial depth texture asset (Startup phase).
    ///
    /// Allocates a depth texture matching the window size with `RENDER_ATTACHMENT | TEXTURE_BINDING`
    /// usage so it can be bound as both a render target and sampled in shaders.
    pub fn init(mut commands: Commands, server: Res<AssetServer>, window: Query<&Window>) {
        // Create the depth texture
        let resolution = &window.single().unwrap().resolution;
        let texture = server.add(Texture {
            label: "depth".to_string(),
            size: (resolution.physical_width(), resolution.physical_height()),
            format: DEPTH_FORMAT,
            usages: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            sample_count: MSAA_SAMPLE_COUNT,
            ..Default::default()
        });

        // Create the depth texture bind group and layout


        // Insert the resources
        commands.insert_resource(DepthTextureMSAA { texture, resized: false });
    }

    /// Recreate the depth texture when the window resizes (Update phase).
    ///
    /// Listens to window resize events and creates a new depth texture asset with the new
    /// surface dimensions. Sets the `resized` flag so the bind group is regenerated.
    pub fn resize(
        mut window_resized_events: MessageReader<SurfaceResized>,
        server: Res<AssetServer>, mut textures: ResMut<DepthTextureMSAA>
    ) {
        textures.resized = false;
        for event in window_resized_events.read() {
            // Recreate the depth texture
            let texture = server.add(Texture {
                label: "depth".to_string(),
                size: (event.width, event.height),
                format: DEPTH_FORMAT,
                usages: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                sample_count: MSAA_SAMPLE_COUNT,
                ..Default::default()
            });

            // Insert the resources
            textures.texture = texture;
            textures.resized = true;
        }
    }

    /// Extract the depth texture handle to the render world (Extract phase).
    ///
    /// Copies the texture handle and resized flag into the render world, and marks
    /// the depth bind group for regeneration if the texture was resized.
    pub fn extract(mut commands: Commands, depth_texture : ExtractWorld<Res<DepthTextureMSAA>>, mut depth_texture_layout: ResMut<DepthMSAATextureLayout>) {
        // If the depth was resized, mark the bind group for rebuild
        if depth_texture.resized {
            depth_texture_layout.layout = None;
            depth_texture_layout.bind_group = None;
        }

        // Insert the depth texture resource
        commands.insert_resource(DepthTextureMSAA {
            texture: depth_texture.texture.clone(),
            resized: false
        });
    }
}
