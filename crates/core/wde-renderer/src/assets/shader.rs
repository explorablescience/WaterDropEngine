use wde_logger::prelude::*;

use bevy::{
    asset::{AssetLoader, LoadContext, io::Reader},
    prelude::*
};
use std::collections::HashSet;
use std::io::{Error, ErrorKind};
use thiserror::Error;

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
    Io(#[from] std::io::Error),
    #[error("Failed to resolve shader include: {0}")]
    Include(String)
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

        // Read the shader data
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

        // Resolve #include directives
        let mut included = HashSet::new();
        let content = resolve_includes(content, load_context, &mut included).await?;
        Ok(Shader { content })
    }

    fn extensions(&self) -> &[&str] {
        &["wgsl"]
    }
}

/// Resolve `#include "path"` directives in `source`, inlining the content of each referenced file.
///
/// Paths are relative to the asset root (`res/`). Each file is included at most once per
/// compilation unit — duplicate `#include` lines for the same path are silently skipped,
/// which also prevents infinite loops from circular includes.
///
/// Included files are registered as hot-reload dependencies: editing an include file
/// triggers a reload of every shader that (directly or transitively) includes it.
async fn resolve_includes(
    source: String,
    load_context: &mut LoadContext<'_>,
    included: &mut HashSet<String>
) -> Result<String, ShaderLoaderError> {
    // Work on a mutable list of lines so we can splice included content in-place.
    // The cursor `i` never skips newly inserted lines, so nested includes are processed
    // automatically without recursion.
    let mut lines: Vec<String> = source.lines().map(str::to_owned).collect();
    let mut i = 0;
    while i < lines.len() {
        let trimmed = lines[i].trim().to_owned();
        if let Some(rest) = trimmed.strip_prefix("#include") {
            let path_str = rest.trim();
            if path_str.starts_with('"') && path_str.ends_with('"') && path_str.len() >= 2 {
                let include_path = path_str[1..path_str.len() - 1].to_owned();
                if !included.contains(&include_path) {
                    included.insert(include_path.clone());
                    let bytes = load_context
                        .read_asset_bytes(include_path.clone())
                        .await
                        .map_err(|e| {
                            ShaderLoaderError::Include(format!(
                                "Could not read '{}': {}",
                                include_path, e
                            ))
                        })?;
                    let include_source =
                        String::from_utf8(bytes).map_err(|_| {
                            ShaderLoaderError::Include(format!(
                                "'{}' is not valid UTF-8",
                                include_path
                            ))
                        })?;
                    let new_lines: Vec<String> =
                        include_source.lines().map(str::to_owned).collect();
                    // Replace the #include line with the file contents.
                    // The cursor stays at `i` so nested #include lines in the
                    // inserted block are processed on the next iterations.
                    lines.splice(i..=i, new_lines);
                } else {
                    // Already included — remove this directive and advance.
                    lines.remove(i);
                }
                continue;
            } else {
                return Err(ShaderLoaderError::Include(format!(
                    "Invalid #include syntax on line {i}: expected `#include \"path\"`"
                )));
            }
        }
        i += 1;
    }
    Ok(lines.join("\n"))
}
