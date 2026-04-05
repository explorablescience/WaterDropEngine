use std::io::{Error, ErrorKind};

use bevy::{
    asset::{AssetLoader, LoadContext, io::Reader},
    prelude::*
};
use thiserror::Error;
use wde_logger::prelude::*;

/// Stores a shader source as UTF-8 text. File should have a `.wgsl` extension.
/// Most of the time, the user does not need to load shaders directly, as they can be embedded in materials and pipelines.
/// Note: the shader will only be compiled on the GPU when used in a pipeline, so this is just a container for the source code.
#[derive(Asset, TypePath, Clone, Debug)]
pub struct Shader {
    /// WGSL source contents as UTF-8 text.
    pub content: String
}

#[derive(Debug, Error)]
pub(crate) enum ShaderLoaderError {
    #[error("Could not load shader: {0}")]
    Io(#[from] std::io::Error)
}
#[derive(Default, TypePath)]
pub(crate) struct ShaderLoader;
impl AssetLoader for ShaderLoader {
    type Asset = Shader;
    type Settings = ();
    type Error = ShaderLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>
    ) -> Result<Self::Asset, Self::Error> {
        debug!("Loading shader {}.", load_context.path());

        // Read the texture data
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;

        // Read the content
        let content = match String::from_utf8(bytes) {
            Ok(content) => content,
            Err(_) => {
                return Err(ShaderLoaderError::Io(Error::new(
                    ErrorKind::InvalidData,
                    "Could not convert shader to string."
                )));
            }
        };
        Ok(Shader { content })
    }

    fn extensions(&self) -> &[&str] {
        &["wgsl"]
    }
}
