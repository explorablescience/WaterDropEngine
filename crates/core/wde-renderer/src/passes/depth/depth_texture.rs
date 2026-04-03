//! Shared depth texture lifecycle and bind group binding.
//!
//! The depth texture is created at startup, resized on window events, extracted to the render
//! world, and made available as a bind group in the render phase. Most passes can bind this
//! depth resource for sampling in fragment shaders or depth testing.

use bevy::prelude::*;
use wde_wgpu::texture::{DEPTH_FORMAT, TextureUsages};

use crate::{assets::Texture, core::{extract_macros::ExtractWorld, window::SurfaceResized}};

/// Holds the handle to the depth texture asset in the main and render worlds.
///
/// The texture is created in Startup, resized on window events, and extracted to the
/// render world during the Extract schedule. The handle references a [`Texture`] asset
/// that is uploaded to GPU asynchronously by the render assets pipeline.
#[derive(Resource)]
pub struct DepthTexture {
    /// Asset handle pointing to the depth texture (CPU/GPU representation).
    pub texture: Handle<Texture>,
    /// Flag set to true when the texture was resized this frame.
    pub resized: bool
}
impl DepthTexture {
    /// Create the initial depth texture asset (Startup phase).
    ///
    /// Allocates a depth texture matching the window size with `RENDER_ATTACHMENT | TEXTURE_BINDING`
    /// usage so it can be bound as both a render target and sampled in shaders.
    pub fn create_texture(mut commands: Commands, server: Res<AssetServer>, window: Query<&Window>) {
        let resolution = &window.single().unwrap().resolution;
        let texture = server.add(Texture {
            label: "depth".to_string(),
            size: (resolution.physical_width(), resolution.physical_height()),
            format: DEPTH_FORMAT,
            usages: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            ..Default::default()
        });
        commands.insert_resource(DepthTexture { texture, resized: false });
    }

    /// Recreate the depth texture when the window resizes (Update phase).
    ///
    /// Listens to window resize events and creates a new depth texture asset with the new
    /// surface dimensions. Sets the `resized` flag so the bind group is regenerated.
    pub fn resize_texture(
        mut window_resized_events: MessageReader<SurfaceResized>,
        server: Res<AssetServer>, mut textures: ResMut<DepthTexture>
    ) {
        textures.resized = false;
        for event in window_resized_events.read() {
            // Recreate the depth texture
            let texture = server.add(Texture {
                label: "depth".to_string(),
                size: (event.width, event.height),
                format: DEPTH_FORMAT,
                usages: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
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
    pub fn extract_texture(mut commands: Commands, depth_texture : ExtractWorld<Res<DepthTexture>>) {
        commands.insert_resource(DepthTexture {
            texture: depth_texture.texture.clone(),
            resized: false
        });
    }
}
