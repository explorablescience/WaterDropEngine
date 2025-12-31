use bevy::{asset::{io::Reader, AssetLoader, LoadContext}, prelude::*};
use serde::{Deserialize, Serialize};
use thiserror::Error;


#[derive(Asset, TypePath, Clone)]
pub struct Shader {
    pub content: String
}

#[derive(Default)]
pub struct ShaderLoader;

#[derive(Serialize, Deserialize, Default)]
pub struct ShaderLoaderSettings {
}

#[derive(Debug, Error)]
pub enum ShaderLoaderError {
    #[error("Could not load shader: {0}")]
    Io(#[from] std::io::Error),
}

impl AssetLoader for ShaderLoader {
    type Asset = Shader;
    type Settings = ();
    type Error = ShaderLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        debug!("Loading shader on the CPU from {}.", load_context.asset_path());

        // Read the texture data
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;

        // Read the content
        let content = match String::from_utf8(bytes) {
            Ok(content) => content,
            Err(_) => return Err(ShaderLoaderError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, "Could not convert shader to string"))),
        };

        Ok(Shader {
            content
        })
    }

    fn extensions(&self) -> &[&str] {
        &["wgsl"]
    }
}

