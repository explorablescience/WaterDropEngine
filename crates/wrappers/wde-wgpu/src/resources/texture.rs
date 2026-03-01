//! Texture creation and copy helpers built on `wgpu::Texture`.

use wde_logger::prelude::*;

use crate::RenderInstanceData;

/// Surface texture.
pub type SurfaceTexture = wgpu::SurfaceTexture;

/// Texture view
pub type TextureView = wgpu::TextureView;

/// Texture usages.
pub type TextureUsages = wgpu::TextureUsages;

/// Texture format.
pub type TextureFormat = wgpu::TextureFormat;


/// The swapchain texture format.
pub const SWAPCHAIN_FORMAT: TextureFormat = TextureFormat::Bgra8UnormSrgb;
/// The depth texture format.
pub const DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32Float;

/// Texture wrapper with a ready-to-use view and sampler.
///
/// # Examples
/// Create a render target and clear it:
/// ```rust,no_run
/// use wde_wgpu::{texture::{Texture, TextureFormat, TextureUsages}, instance::RenderInstanceData};
/// 
/// let color = Texture::new(
///     instance,
///     "color-target",
///     (1280, 720),
///     TextureFormat::Rgba8Unorm,
///     TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
/// );
/// ```
///
/// Upload raw pixel data (RGBA8):
/// ```rust,no_run
/// use wde_wgpu::{texture::{Texture, TextureFormat, TextureUsages}, instance::RenderInstanceData};
/// 
/// let texture = Texture::new(
///     instance,
///     "albedo",
///     (512, 512),
///     TextureFormat::Rgba8Unorm,
///     TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
/// );
/// texture.copy_from_buffer(instance, TextureFormat::Rgba8Unorm, pixels);
/// // Or
/// texture.copy_from_buffer_layered(instance, TextureFormat::Rgba8Unorm, 0, pixels); // For texture arrays
/// ```
///
/// Copy one GPU texture into another:
/// ```rust,no_run
/// # use wde_wgpu::{texture::{Texture, TextureFormat, TextureUsages}, instance::RenderInstanceData};
/// 
/// let dst = Texture::new(
///     instance,
///     "blit-target",
///     (1024, 1024),
///     TextureFormat::Rgba8Unorm,
///     TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
/// );
/// dst.copy_from_texture(instance, src, (1024, 1024));
/// ```
pub struct Texture {
    pub label: String,
    pub texture: wgpu::Texture,
    pub format: TextureFormat,
    pub view: TextureView,
    pub sampler: wgpu::Sampler,
    pub size: (u32, u32),
    pub sample_count: u32,
    pub layer_count: u32,
}

impl std::fmt::Debug for Texture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Texture")
            .field("label", &self.label)
            .field("sampler", &self.sampler)
            .field("size", &self.size)
            .field("sample_count", &self.sample_count)
            .field("layer_count", &self.layer_count)
            .finish()
    }
}

impl Texture {
    /// Create a new texture.
    /// 
    /// # Arguments
    /// 
    /// * `instance` - Game instance.
    /// * `label` - Label of the texture. This is only for debugging purposes.
    /// * `size` - Size of the texture (width, height).
    /// * `format` - Format of the texture (e.g. Rgba8Unorm, Depth32Float, etc.).
    /// * `usage` - Usage of the texture (e.g. RENDER_ATTACHMENT, COPY_SRC, COPY_DST, etc.).
    /// * `sample_count` - Sample count of the texture (e.g. 1 for no MSAA, 4 for 4x MSAA, etc.).
    /// * `layer_count` - Number of layers in the texture array. Default is 1 (for non-array textures).
    pub fn new(instance: &RenderInstanceData<'_>, label: &str, size: (u32, u32), format: TextureFormat, usage: TextureUsages, sample_count: u32, layer_count: u32) -> Self {
        event!(Level::DEBUG, "Creating wgpu texture {}.", label);
        
        // Create texture
        let texture = instance.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(format!("{}-texture", label).as_str()),
            size: wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: layer_count,
            },
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: usage | wgpu::TextureUsages::COPY_DST,
            view_formats: &[]
        });

        // Create texture view
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some(format!("{}-texture-view", label).as_str()),
            format: if format == DEPTH_FORMAT {
                None
            } else {
                Some(format)
            },
            dimension: if format == DEPTH_FORMAT {
                None
            } else if layer_count > 1 {
                Some(wgpu::TextureViewDimension::D2Array)
            } else {
                Some(wgpu::TextureViewDimension::D2)
            },
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            base_array_layer: 0,
            mip_level_count: None,
            array_layer_count: if layer_count > 1 { Some(layer_count) } else { None },
            usage: None,
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
            sample_count,
            layer_count
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
    pub fn copy_from_buffer(&self, instance: &RenderInstanceData, texture_format: TextureFormat, buffer: &[u8]) {
        event!(Level::TRACE, "Copying buffer to texture.");

        // Retrieve size corresponding to the texture format
        let format_size = match texture_format.block_dimensions() {
            (1, 1) => texture_format.block_copy_size(None).unwrap() as usize,
            _ => panic!("Using pixel_size for compressed textures is invalid"),
        };

        // Copy buffer to texture
        instance.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            buffer,
            wgpu::TexelCopyBufferLayout {
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


    /// Copy texture to buffer at a given array layer.
    /// It is assumed that the buffer is the same size as the texture.
    /// It will be copied on the next queue submit.
    /// Note that the buffer must have the COPY_DST usage.
    /// 
    /// # Arguments
    /// 
    /// * `instance` - Game instance.
    /// * `texture_format` - The wgpu texture format.
    /// * `array_layer` - The array layer to copy from (for texture arrays). Default is 0 for non-array textures.
    /// * `buffer` - Image buffer.
    pub fn copy_from_buffer_layered(&self, instance: &RenderInstanceData, texture_format: TextureFormat, array_layer: u32, buffer: &[u8]) {
        event!(Level::TRACE, "Copying texture to buffer.");

        // Retrieve size corresponding to the texture format
        let format_size = match texture_format.block_dimensions() {
            (1, 1) => texture_format.block_copy_size(None).unwrap() as usize,
            _ => panic!("Using pixel_size for compressed textures is invalid"),
        };

        // Copy buffer to texture
        instance.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: 0,
                    y: 0,
                    z: array_layer,
                },
                aspect: wgpu::TextureAspect::All,
            },
            buffer,
            wgpu::TexelCopyBufferLayout {
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
    pub fn copy_from_texture(&self, instance: &RenderInstanceData<'_>, texture: &wgpu::Texture, size: (u32, u32)) {
        event!(Level::TRACE, "Copying texture to texture.");

        // Create command buffer
        let mut command = crate::command_buffer::CommandBuffer::new(instance, "Copy Texture");

        // Copy texture to texture
        command.encoder().copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyTextureInfo {
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

    /// Copy texture to texture at a given array layer.
    /// It is assumed that the texture is the same size as the source texture.
    /// Note that the input texture must have the COPY_SRC usage, and the output texture must have the COPY_DST usage.
    /// 
    /// # Arguments
    /// 
    /// * `instance` - Game instance.
    /// * `texture` - Texture to copy from.
    /// * `array_layer` - The array layer to copy to (for texture arrays). Default is 0 for non-array textures.
    /// * `size` - Size of the texture.
    pub fn copy_from_texture_layered(&self, instance: &RenderInstanceData<'_>, texture: &wgpu::Texture, array_layer: usize, size: (u32, u32)) {
        event!(Level::TRACE, "Copying texture to texture.");

        // Create command buffer
        let mut command = crate::command_buffer::CommandBuffer::new(instance, "Copy Texture");

        // Copy texture to texture
        command.encoder().copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: 0,
                    y: 0,
                    z: array_layer as u32,
                },
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
