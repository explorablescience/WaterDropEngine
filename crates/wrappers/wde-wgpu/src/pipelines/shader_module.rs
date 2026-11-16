//! WGSL shader modules of the render and compute pipelines.
use futures_lite::future;
use wde_logger::prelude::*;
use wgpu::naga;

use crate::instance::{RenderError, RenderInstanceData};

/// Parses, validates and creates a WGSL shader module. The error holds the messages, with the source of each
/// problem.
pub(crate) fn create_shader_module(
    instance: &RenderInstanceData,
    label: &str,
    stage: &str,
    source: &str
) -> Result<wgpu::ShaderModule, RenderError> {
    let path = format!("{label} ({stage} shader)");
    let failed = |message: String| RenderError::ShaderCompilationError(message);
    let module = naga::front::wgsl::parse_str(source)
        .map_err(|e| failed(e.emit_to_string_with_path(source, &path)))?;
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all()
    )
    .validate(&module)
    .map_err(|e| failed(e.emit_to_string_with_path(source, &path)))?;

    let error_scope = instance
        .device
        .push_error_scope(wgpu::ErrorFilter::Validation);
    let shader_module = instance
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(&format!("{label}-{stage}")),
            source: wgpu::ShaderSource::Wgsl(source.into())
        });
    let mut errors = Vec::new();
    future::block_on(async {
        for message in shader_module.get_compilation_info().await.messages {
            let text = format!("{path}: {} (at {:?})", message.message, message.location);
            match message.message_type {
                wgpu::CompilationMessageType::Error => errors.push(text),
                wgpu::CompilationMessageType::Warning => warn!("{text}."),
                wgpu::CompilationMessageType::Info => debug!("{text}.")
            }
        }
        if let Some(error) = error_scope.pop().await {
            errors.push(format!("{path}: {error}"));
        }
    });
    if errors.is_empty() {
        Ok(shader_module)
    } else {
        Err(failed(errors.join("\n")))
    }
}

/// Error of the pipeline created in `create`, reported by a validation error scope.
pub(crate) fn validate_pipeline<T>(
    instance: &RenderInstanceData,
    create: impl FnOnce() -> T
) -> Result<T, RenderError> {
    let error_scope = instance
        .device
        .push_error_scope(wgpu::ErrorFilter::Validation);
    let pipeline = create();
    match future::block_on(error_scope.pop()) {
        Some(error) => Err(RenderError::PipelineCreationError(error.to_string())),
        None => Ok(pipeline)
    }
}
