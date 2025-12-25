//! Contains the texture struct and its implementations.

use bevy::{log::Level, log::tracing::event};
use wgpu::TextureFormat;

use crate::instance::WRenderInstanceData;

/// Surface texture.
pub type WSurfaceTexture = wgpu::SurfaceTexture;

/// Texture view
pub type WTextureView = wgpu::TextureView;

/// Texture usages.
pub type WTextureUsages = wgpu::TextureUsages;

/// Texture format.
pub type WTextureFormat = wgpu::TextureFormat;

/// Texture struct.
/// 
/// # Example
/// 
/// ```
/// // Create a new texture
/// let texture = WTexture::new(&instance,
///     "Texture Label", (1024, 1024), TextureFormat::Rgba8Unorm,
///     TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC);
/// 
/// // Copy buffer to texture
/// texture.copy_from_buffer(&instance, &buffer, false);
/// 
/// // Copy texture to texture
/// texture.copy_from_texture(&instance, &texture, (1024, 1024));
/// ```
pub struct WTexture {
    pub label: String,
    pub texture: wgpu::Texture,
    pub format: WTextureFormat,
    pub view: WTextureView,
    pub sampler: wgpu::Sampler,
    pub size: (u32, u32),
}

impl std::fmt::Debug for WTexture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Texture")
            .field("label", &self.label)
            .field("sampler", &self.sampler)
            .field("size", &self.size)
            .finish()
    }
}

impl WTexture {
    /// The swap chain texture format.
    pub const SWAPCHAIN_FORMAT: WTextureFormat = WTextureFormat::Bgra8UnormSrgb;
    /// The depth texture format.
    pub const DEPTH_FORMAT: WTextureFormat = WTextureFormat::Depth32Float;

    /// Create a new texture.
    /// 
    /// # Arguments
    /// 
    /// * `instance` - Game instance.
    /// * `label` - Label of the texture.
    /// * `size` - Size of the texture.
    /// * `format` - Format of the texture.
    /// * `usage` - Usage of the texture.
    pub fn new(instance: &WRenderInstanceData<'_>, label: &str, size: (u32, u32), format: WTextureFormat, usage: WTextureUsages) -> Self {
        event!(Level::DEBUG, "Creating wgpu texture {}.", label);
        
        // Create texture
        let texture = instance.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(format!("{}-texture", label).as_str()),
            size: wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: usage | wgpu::TextureUsages::COPY_DST,
            view_formats: &[]
        });

        // Create texture view
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some(format!("{}-texture-view", label).as_str()),
            format: if format == Self::DEPTH_FORMAT {
                None
            } else {
                Some(format)
            },
            dimension: if format == Self::DEPTH_FORMAT {
                None
            } else {
                Some(wgpu::TextureViewDimension::D2)
            },
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            base_array_layer: 0,
            mip_level_count: None,
            array_layer_count: None
        });

        // Create sampler
        let sampler = instance.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some(format!("{}-texture-sampler", label).as_str()),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            compare: None,
            anisotropy_clamp: 1,
            border_color: None,
        });

        // Return texture
        Self {
            label: label.to_string(),
            texture,
            format,
            view,
            sampler,
            size,
        }
    }


    /// Copy buffer to texture.
    /// It is assumed that the buffer is the same size as the texture.
    /// It will be copied on the next queue submit.
    /// Note that the buffer must have the COPY_DST usage.
    /// 
    /// # Arguments
    /// 
    /// * `instance` - Game instance.
    /// * `texture_format` - The wgpu texture format.
    /// * `buffer` - Image buffer.
    pub fn copy_from_buffer(&self, instance: &WRenderInstanceData, texture_format: TextureFormat, buffer: &[u8]) {
        event!(Level::TRACE, "Copying buffer to texture.");

        // Retrieve size corresponding to the texture format
        let format_size = match texture_format.block_dimensions() {
            (1, 1) => texture_format.block_copy_size(None).unwrap() as usize,
            _ => panic!("Using pixel_size for compressed textures is invalid"),
        };

        // Copy buffer to texture
        instance.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            buffer,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(self.size.0 * format_size as u32),
                rows_per_image: None,
            },
            wgpu::Extent3d {
                width: self.size.0,
                height: self.size.1,
                depth_or_array_layers: 1,
            },
        );
    } 
    
    /// Copy texture to texture.
    /// It is assumed that the texture is the same size as the source texture.
    /// Note that the input texture must have the COPY_SRC usage, and the output texture must have the COPY_DST usage.
    /// 
    /// # Arguments
    /// 
    /// * `instance` - Game instance.
    /// * `texture` - Texture to copy from.
    /// * `size` - Size of the texture.
    pub fn copy_from_texture(&self, instance: &WRenderInstanceData<'_>, texture: &wgpu::Texture, size: (u32, u32)) {
        event!(Level::TRACE, "Copying texture to texture.");

        // Create command buffer
        let mut command = crate::command_buffer::WCommandBuffer::new(instance, "Copy Texture");

        // Copy texture to texture
        command.encoder().copy_texture_to_texture(
            wgpu::ImageCopyTexture {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyTexture {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
        );

        // Submit the commands
        command.submit(instance);
    }
}
