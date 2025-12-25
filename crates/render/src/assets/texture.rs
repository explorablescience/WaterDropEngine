use bevy::{asset::{io::Reader, AssetLoader, LoadContext}, ecs::system::lifetimeless::SRes, prelude::*, log::tracing::error};
use image::GenericImageView;
use thiserror::Error;
use serde::{Deserialize, Serialize};
use wde_wgpu::{instance::WRenderInstance, texture::{WTextureFormat, WTextureUsages}};

use super::render_assets::{PrepareAssetError, RenderAsset};


#[derive(Asset, TypePath, Clone)]
pub struct Texture {
    pub label: String,
    pub size: (u32, u32),
    pub format: WTextureFormat,
    pub usages: WTextureUsages,
    pub data: Vec<u8>
}
impl Default for Texture {
    fn default() -> Self {
        Texture {
            label: "Texture".to_string(),
            size: (1, 1),
            format: WTextureFormat::Rgba8Unorm,
            usages: WTextureUsages::TEXTURE_BINDING,
            data: Vec::new()
        }
    }
}

#[derive(Default)]
pub struct TextureLoader;

#[derive(Serialize, Deserialize)]
pub struct TextureLoaderSettings {
    /// The label of the texture.
    pub label: String,
    /// The format of the texture (by default RGBA8Unorm).
    pub format: WTextureFormat,
    /// The usages of the texture (by default TEXTURE_BINDING).
    pub usages: WTextureUsages
}

impl Default for TextureLoaderSettings {
    fn default() -> Self {
        Self {
            label: "texture".to_string(),
            format: WTextureFormat::Rgba8Unorm,
            usages: WTextureUsages::TEXTURE_BINDING
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
        debug!("Loading texture on the CPU from {}.", load_context.asset_path());

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
            data
        })
    }

    fn extensions(&self) -> &[&str] {
        &["png", "jpg"]
    }
}



pub struct GpuTexture {
    pub label: String,
    pub texture: wde_wgpu::texture::WTexture,
}
impl RenderAsset for GpuTexture {
    type SourceAsset = Texture;
    type Param = SRes<WRenderInstance<'static>>;

    fn prepare_asset(
            asset: Self::SourceAsset,
            render_instance: &mut bevy::ecs::system::SystemParamItem<Self::Param>,
        ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        debug!(asset.label, "Loading texture on the GPU.");

        let render_instance = render_instance.data.as_ref().read().unwrap();

        // Create the texture
        let texture = wde_wgpu::texture::WTexture::new(
            &render_instance, &asset.label, (asset.size.0, asset.size.1),
            asset.format, asset.usages);

        // Copy the texture data
        if !asset.data.is_empty() {
            texture.copy_from_buffer(&render_instance, asset.format, &asset.data);
        }

        Ok(GpuTexture { label: asset.label, texture })
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
fn get_format_properties(texture_format: WTextureFormat) -> Option<(u32, u32)> {
    match texture_format {
        WTextureFormat::R8Unorm | WTextureFormat::R8Uint | WTextureFormat::R8Snorm | WTextureFormat::R8Sint => {
            Some((8, 1))
        },
        WTextureFormat::R16Unorm | WTextureFormat::R16Uint | WTextureFormat::R16Snorm | WTextureFormat::R16Sint | WTextureFormat::R16Float => {
            Some((16, 1))
        },
        WTextureFormat::R32Uint | WTextureFormat::R32Sint | WTextureFormat::R32Float => {
            Some((32, 1))
        },
        WTextureFormat::Rg8Unorm | WTextureFormat::Rg8Uint | WTextureFormat::Rg8Snorm | WTextureFormat::Rg8Sint => {
            Some((8, 2))
        },
        WTextureFormat::Rg16Unorm | WTextureFormat::Rg16Uint | WTextureFormat::Rg16Snorm | WTextureFormat::Rg16Sint | WTextureFormat::Rg16Float => {
            Some((16, 2))
        },
        WTextureFormat::Rg32Uint | WTextureFormat::Rg32Sint | WTextureFormat::Rg32Float => {
            Some((32, 2))
        },
        WTextureFormat::Rgba8Unorm | WTextureFormat::Rgba8UnormSrgb | WTextureFormat::Rgba8Uint | WTextureFormat::Rgba8Snorm | WTextureFormat::Rgba8Sint => {
            Some((8, 4))
        },
        WTextureFormat::Rgba16Unorm | WTextureFormat::Rgba16Uint | WTextureFormat::Rgba16Snorm | WTextureFormat::Rgba16Sint | WTextureFormat::Rgba16Float => {
            Some((16, 4))
        },
        WTextureFormat::Rgba32Uint | WTextureFormat::Rgba32Sint | WTextureFormat::Rgba32Float => {
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
