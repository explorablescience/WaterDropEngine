//! CPU and GPU texture assets plus the Bevy loader wiring used by the renderer.
//!
//! The CPU-side [`Texture`] mirrors `wde-wgpu` conventions: explicit labels for
//! debugging, explicit format/usage flags, and an optional pixel payload. The
//! loader reads image files into that buffer, while [`GpuTexture`] turns the
//! asset into a GPU texture inside the render sub-app.

use wde_logger::prelude::*;
use bevy::{asset::{io::Reader, AssetLoader, LoadContext}, ecs::system::lifetimeless::SRes, prelude::*};
use image::GenericImageView;
use thiserror::Error;
use serde::{Deserialize, Serialize};

use crate::core::RenderInstance;

use super::render_assets::{PrepareAssetError, RenderAsset};

// Reexport structs
pub use wde_wgpu::texture::{TextureFormat, TextureUsages, SWAPCHAIN_FORMAT, DEPTH_FORMAT};

#[derive(Asset, TypePath, Clone)]
pub struct Texture {
    /// Human readable identifier applied to the GPU resource label.
    pub label: String,
    /// Width and height in pixels of the CPU buffer.
    pub size: (u32, u32),
    /// GPU texture format requested when allocating the resource.
    pub format: TextureFormat,
    /// GPU usage flags (sampling, storage, copy, etc.).
    pub usages: TextureUsages,
    /// Sample count for the texture (1 for no MSAA, etc.).
    pub sample_count: u32,
    /// Number of layers for array textures (1 for non-arrays, etc.).
    pub layer_count: u32,
    /// Number of mip levels (1 = no mipmaps, 0 = auto-calculate max levels).
    pub mip_level_count: u32,
    /// Raw pixel payload matching `format`; empty means allocate-only.
    pub data: Vec<u8>
}
impl Default for Texture {
    fn default() -> Self {
        Texture {
            label: "Texture".to_string(),
            size: (1, 1),
            format: TextureFormat::Rgba8Unorm,
            usages: TextureUsages::TEXTURE_BINDING,
            sample_count: 1,
            layer_count: 1,
            mip_level_count: 1,
            data: Vec::new()
        }
    }
}

#[derive(Default, TypePath)]
/// Bevy asset loader that decodes images and fills a CPU-side [`Texture`].
pub struct TextureLoader;

#[derive(Serialize, Deserialize)]
pub struct TextureLoaderSettings {
    /// Label propagated to the CPU and GPU resource.
    pub label: String,
    /// Requested GPU format (defaults to `Rgba8Unorm`).
    pub format: TextureFormat,
    /// Usage flags applied when creating the GPU texture (defaults to `TEXTURE_BINDING`).
    pub usages: TextureUsages
}

impl Default for TextureLoaderSettings {
    fn default() -> Self {
        Self {
            label: "texture".to_string(),
            format: TextureFormat::Rgba8Unorm,
            usages: TextureUsages::TEXTURE_BINDING
        }
    }
}

#[derive(Debug, Error)]
pub enum TextureLoaderError {
    #[error("Could not load texture: {0}")]
    Io(#[from] std::io::Error),
}

impl AssetLoader for TextureLoader {
    type Asset = Texture;
    type Settings = TextureLoaderSettings;
    type Error = TextureLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        settings: &TextureLoaderSettings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        debug!("Loading texture on the CPU from {}.", load_context.path());

        // Read the texture data bytes
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;

        // Load the image
        let image = match image::load_from_memory(&bytes) {
            Ok(image) => image,
            Err(err) => {
                error!("Could not load texture: {}", err);
                return Err(TextureLoaderError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, err)));
            }
        };
        let size = image.dimensions();

        // Convert to right format pixel size
        let format_properties = get_format_properties(settings.format).unwrap();
        let data = match format_properties.0 {
            8  => from_channels(&image.into_rgba8(), format_properties.1),
            16 => bytemuck::cast_slice(&from_channels(&image.into_rgba16(),  format_properties.1)).to_vec(),
            21 => bytemuck::cast_slice(&from_channels(&image.into_rgba32f(), format_properties.1)).to_vec(),
            _ => unreachable!()
        };

        Ok(Texture {
            label: settings.label.clone(),
            format: settings.format,
            usages: settings.usages,
            size,
            sample_count: 1,
            layer_count: 1,
            mip_level_count: 1,
            data
        })
    }

    fn extensions(&self) -> &[&str] {
        &["png", "jpg"]
    }
}



pub struct GpuTexture {
    /// Copy of the CPU label applied to the GPU handle.
    pub label: String,
    /// GPU texture handle allocated through `wde-wgpu`.
    pub texture: wde_wgpu::texture::Texture,
    /// Whether the bind group using this texture needs to be rebuilt.
    pub dirty: bool
}
impl RenderAsset for GpuTexture {
    type SourceAsset = Texture;
    type Param = SRes<RenderInstance>;

    fn prepare_asset(
            asset: Self::SourceAsset,
            render_instance: &mut bevy::ecs::system::SystemParamItem<Self::Param>,
        ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        debug!("Uploading texture '{}' to the GPU.", asset.label);

        let render_instance = render_instance.0.as_ref().read().unwrap();

        // Create the texture with mip levels
        let texture = wde_wgpu::texture::Texture::new(
            &render_instance, &asset.label, (asset.size.0, asset.size.1),
            asset.format, asset.usages, asset.sample_count, asset.layer_count, asset.mip_level_count
        );

        // Copy the texture data
        if !asset.data.is_empty() {
            texture.copy_from_buffer(&render_instance, asset.format, &asset.data);
        }

        Ok(GpuTexture { label: asset.label, texture, dirty: true })
    }

    fn label(&self) -> &str {
        &self.label
    }
}


/// Get the properties of a texture format.
/// 
/// # Returns
/// 
/// - `None` if the format is not supported.
/// - `Some` with the properties of the format:
///    - `bits`: Can be 8 bits, 16 bits or 32 bits.
///    - `channels`: The number of channels in the format (1 to 4).
fn get_format_properties(texture_format: TextureFormat) -> Option<(u32, u32)> {
    match texture_format {
        TextureFormat::R8Unorm | TextureFormat::R8Uint | TextureFormat::R8Snorm | TextureFormat::R8Sint => {
            Some((8, 1))
        },
        TextureFormat::R16Unorm | TextureFormat::R16Uint | TextureFormat::R16Snorm | TextureFormat::R16Sint | TextureFormat::R16Float => {
            Some((16, 1))
        },
        TextureFormat::R32Uint | TextureFormat::R32Sint | TextureFormat::R32Float => {
            Some((32, 1))
        },
        TextureFormat::Rg8Unorm | TextureFormat::Rg8Uint | TextureFormat::Rg8Snorm | TextureFormat::Rg8Sint => {
            Some((8, 2))
        },
        TextureFormat::Rg16Unorm | TextureFormat::Rg16Uint | TextureFormat::Rg16Snorm | TextureFormat::Rg16Sint | TextureFormat::Rg16Float => {
            Some((16, 2))
        },
        TextureFormat::Rg32Uint | TextureFormat::Rg32Sint | TextureFormat::Rg32Float => {
            Some((32, 2))
        },
        TextureFormat::Rgba8Unorm | TextureFormat::Rgba8UnormSrgb | TextureFormat::Rgba8Uint | TextureFormat::Rgba8Snorm | TextureFormat::Rgba8Sint => {
            Some((8, 4))
        },
        TextureFormat::Rgba16Unorm | TextureFormat::Rgba16Uint | TextureFormat::Rgba16Snorm | TextureFormat::Rgba16Sint | TextureFormat::Rgba16Float => {
            Some((16, 4))
        },
        TextureFormat::Rgba32Uint | TextureFormat::Rgba32Sint | TextureFormat::Rgba32Float => {
            Some((32, 4))
        },
        _ => None
    }
}

/// Convert an image to a pixel buffer.
fn from_channels<T: Clone + Copy + bytemuck::NoUninit + bytemuck::Pod>(data: &[T], channels: u32) -> Vec<T> {
    let inv_channels = [4, 3, 2, 1];
    if channels == 4 {
        return data.to_vec();
    }
    let inv_channel = inv_channels[channels as usize - 1];

    // Extract channels
    let mut buffer: Vec<T> = Vec::with_capacity(data.len() / inv_channel as usize);
    for i in 0..data.len() / inv_channel as usize {
        buffer.push(data[i * inv_channel as usize]);
    }
    buffer
}
