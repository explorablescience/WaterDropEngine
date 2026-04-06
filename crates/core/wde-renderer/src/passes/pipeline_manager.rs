use super::{ComputePipelineDescriptor, RenderPipelineDescriptor};
use crate::assets::PrepareAssetError;
use crate::core::RenderInstance;
use crate::{
    assets::Shader,
    core::{Extract, ExtractWorld, Render, RenderSet}
};
use bevy::{
    app::{App, Plugin},
    asset::{AssetEvent, AssetId, Assets},
    ecs::prelude::*,
    prelude::MessageReader
};
use std::collections::HashMap;
use wde_logger::prelude::*;
use wde_wgpu::{
    compute_pipeline::ComputePipeline,
    render_pipeline::{RenderPipeline, ShaderStages}
};

pub(crate) struct PipelineManagerPlugin;
impl Plugin for PipelineManagerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PipelineManager>()
            .add_systems(Extract, extract_shaders)
            .add_systems(
                Render,
                (load_render_pipelines, load_compute_pipelines).in_set(RenderSet::Prepare)
            );
    }
}

/// The status of a cached pipeline. Used to query the pipeline manager for the status of a pipeline.
pub enum CachedPipelineStatus<'a> {
    Loading,
    OkRender(&'a RenderPipeline),
    OkCompute(&'a ComputePipeline),
    Error
}
/// The index of a cached pipeline.
pub type CachedPipelineIndex = usize;
/// Stores queued and realized pipelines plus shader cache.
/// Provides an interface to queue pipelines for loading and query their status.
#[derive(Resource, Default)]
pub struct PipelineManager {
    /// Monotonic index generator for cached pipelines.
    pipeline_iter: CachedPipelineIndex,

    /// Render pipelines waiting to be built.
    processing_render_pipelines: HashMap<CachedPipelineIndex, RenderPipelineDescriptor>,
    /// Built render pipelines ready for use.
    loaded_render_pipelines: HashMap<CachedPipelineIndex, RenderPipeline>,
    /// Original descriptors corresponding to built render pipelines.
    loaded_render_pipelines_desc: HashMap<CachedPipelineIndex, RenderPipelineDescriptor>,

    /// Compute pipelines waiting to be built.
    processing_compute_pipelines: HashMap<CachedPipelineIndex, ComputePipelineDescriptor>,
    /// Built compute pipelines ready for use.
    loaded_compute_pipelines: HashMap<CachedPipelineIndex, ComputePipeline>,
    /// Original descriptors corresponding to built compute pipelines.
    loaded_compute_pipelines_desc: HashMap<CachedPipelineIndex, ComputePipelineDescriptor>,

    /// CPU-side cache of shader assets keyed by asset id.
    shader_cache: HashMap<AssetId<Shader>, Shader>,
    /// Mapping from shader asset ids to pipelines that reference them (for hot-reload).
    shader_to_pipelines: HashMap<AssetId<Shader>, Vec<CachedPipelineIndex>>
}
impl PipelineManager {
    /// Push the creation of a render pipeline to the pipeline manager queue.
    pub fn create_render_pipeline<E: Send + Sync + 'static>(
        &mut self,
        descriptor: RenderPipelineDescriptor,
        asset: E
    ) -> Result<CachedPipelineIndex, PrepareAssetError<E>> {
        // Check that every bind group layout is Some
        for layout in descriptor.bind_group_layouts.iter() {
            if layout.is_none() {
                return Err(PrepareAssetError::RetryNextUpdate(asset));
            }
        }

        // Store the pipeline descriptor to the queued pipelines
        let id = self.pipeline_iter;
        self.processing_render_pipelines.insert(id, descriptor);
        self.pipeline_iter += 1;
        Ok(id)
    }

    /// Push the creation of a compute pipeline to the pipeline manager queue.
    pub fn create_compute_pipeline<E: Send + Sync + 'static>(
        &mut self,
        descriptor: ComputePipelineDescriptor,
        asset: E
    ) -> Result<CachedPipelineIndex, PrepareAssetError<E>> {
        // Check that every bind group layout is Some
        for layout in descriptor.bind_group_layouts.iter() {
            if layout.is_none() {
                return Err(PrepareAssetError::RetryNextUpdate(asset));
            }
        }

        // Store the pipeline descriptor to the queued pipelines
        let id = self.pipeline_iter;
        self.processing_compute_pipelines.insert(id, descriptor);
        self.pipeline_iter += 1;
        Ok(id)
    }

    /// Get the status of a pipeline from its cached index.
    /// Returns loading/ready/error state for both render and compute pipelines.
    pub fn get_pipeline(&'_ self, id: CachedPipelineIndex) -> CachedPipelineStatus<'_> {
        if self.processing_render_pipelines.contains_key(&id)
            || self.processing_compute_pipelines.contains_key(&id)
        {
            CachedPipelineStatus::Loading
        } else if let Some(pipeline) = self.loaded_render_pipelines.get(&id) {
            CachedPipelineStatus::OkRender(pipeline)
        } else if let Some(pipeline) = self.loaded_compute_pipelines.get(&id) {
            CachedPipelineStatus::OkCompute(pipeline)
        } else {
            error_once!("Pipeline with id {} not found", id);
            CachedPipelineStatus::Error
        }
    }
}

/// Extract the shaders from the asset server and store them in the pipeline manager.
fn extract_shaders(
    mut pipeline_manager: ResMut<PipelineManager>,
    shaders: ExtractWorld<Res<Assets<Shader>>>,
    mut shader_events: ExtractWorld<MessageReader<AssetEvent<Shader>>>
) {
    let cache = &mut pipeline_manager.shader_cache;
    let mut updated_ids = Vec::new();
    for event in shader_events.read() {
        match event {
            AssetEvent::Added { id } => {
                if let Some(shader) = shaders.get(*id) {
                    cache.insert(*id, shader.clone());
                }
            }
            AssetEvent::Modified { id } => {
                if let Some(shader) = shaders.get(*id) {
                    cache.insert(*id, shader.clone());
                    updated_ids.push(*id);
                }
            }
            AssetEvent::Removed { id } => {
                cache.remove(id);
            }
            AssetEvent::Unused { .. } => {}
            AssetEvent::LoadedWithDependencies { .. } => {}
        }
    }

    // Recreate the shader to pipelines map
    for id in updated_ids {
        let p_ids = match pipeline_manager.shader_to_pipelines.get(&id) {
            Some(p_ids) => p_ids.clone(),
            None => continue
        };
        for p_id in p_ids.iter() {
            // Only update the pipeline if it is loaded
            if pipeline_manager.loaded_render_pipelines.contains_key(p_id) {
                let desc = pipeline_manager
                    .loaded_render_pipelines_desc
                    .remove(p_id)
                    .unwrap();
                pipeline_manager
                    .processing_render_pipelines
                    .insert(*p_id, desc.clone());
                pipeline_manager.loaded_render_pipelines.remove(p_id);
            }
            if pipeline_manager.loaded_compute_pipelines.contains_key(p_id) {
                let desc = pipeline_manager
                    .loaded_compute_pipelines_desc
                    .remove(p_id)
                    .unwrap();
                pipeline_manager
                    .processing_compute_pipelines
                    .insert(*p_id, desc.clone());
                pipeline_manager.loaded_compute_pipelines.remove(p_id);
            }
        }
    }
}

/// Load the pipelines that are queued in the pipeline manager.
fn load_render_pipelines(
    mut pipeline_manager: ResMut<PipelineManager>,
    render_instance: Res<RenderInstance>
) {
    let mut pipelines_loaded_indices: Vec<(usize, RenderPipeline)> = Vec::new();
    let mut pipelines_loaded_desc: HashMap<CachedPipelineIndex, RenderPipelineDescriptor> =
        HashMap::new();
    let mut shaders_to_pipelines: HashMap<AssetId<Shader>, Vec<CachedPipelineIndex>> =
        pipeline_manager.shader_to_pipelines.clone();
    for (id, descriptor) in pipeline_manager.processing_render_pipelines.iter() {
        let mut can_load = true;

        // Check if vertex shader is loaded
        let vert_shader = match &descriptor.vert {
            Some(shader) => {
                match pipeline_manager.shader_cache.get(&shader.id()) {
                    Some(shader) => Some(shader),
                    None => {
                        // Shader is not loaded yet
                        can_load = false;
                        None
                    }
                }
            }
            None => None
        };

        // Check if fragment shader is loaded
        let frag_shader = match &descriptor.frag {
            Some(shader) => {
                match pipeline_manager.shader_cache.get(&shader.id()) {
                    Some(shader) => Some(shader),
                    None => {
                        // Shader is not loaded yet
                        can_load = false;
                        None
                    }
                }
            }
            None => None
        };

        // Skip if shaders are not loaded
        if !can_load {
            continue;
        }
        pipelines_loaded_desc.insert(*id, descriptor.clone());
        shaders_to_pipelines
            .entry(descriptor.vert.as_ref().unwrap().id())
            .or_default()
            .push(*id);
        shaders_to_pipelines
            .entry(descriptor.frag.as_ref().unwrap().id())
            .or_default()
            .push(*id);

        // Build the layouts
        let mut bind_group_layouts = Vec::new();
        for layout in descriptor.bind_group_layouts.iter() {
            let layout = match layout.as_ref().unwrap().build(&render_instance.0.read().unwrap()) {
                Ok(layout) => layout,
                Err(e) => {
                    error!(
                        "Failed to build bind group layout for pipeline {}: {:?}",
                        descriptor.label, e
                    );
                    continue;
                }
            };
            bind_group_layouts.push(layout);
        }

        // Load the pipeline
        let mut pipeline = RenderPipeline::new(descriptor.label);
        if let Some(vert_shader) = vert_shader {
            pipeline.set_shader(&vert_shader.content, ShaderStages::VERTEX);
        }
        if let Some(frag_shader) = frag_shader {
            pipeline.set_shader(&frag_shader.content, ShaderStages::FRAGMENT);
        }
        pipeline.set_fragment_blend(descriptor.fragment_blend);
        pipeline.set_topology(descriptor.topology);
        pipeline.set_cull_mode(descriptor.cull_mode);
        pipeline.set_depth(descriptor.depth.clone());
        pipeline.set_use_vertices_buffer(descriptor.vertex_buffer);
        if let Some(ref render_targets) = descriptor.render_targets {
            pipeline.set_render_targets(render_targets.clone());
        }
        pipeline.set_sample_count(descriptor.sample_count);
        for push_constant in descriptor.push_constants.iter() {
            pipeline.add_push_constant(
                push_constant.stages,
                push_constant.offset,
                push_constant.size
            );
        }
        pipeline.set_bind_groups(bind_group_layouts);
        match pipeline.init(&render_instance.0.read().unwrap()) {
            Ok(_) => (),
            Err(e) => {
                error_once!("Failed to load pipeline: {:?}", e);
                continue;
            }
        }
        debug!("Loaded pipeline {} with id {}", descriptor.label, id);

        // Add the pipeline to the loaded pipelines
        pipelines_loaded_indices.push((*id, pipeline));
    }

    // Remove loaded pipelines and add them to the loaded pipelines
    while let Some((id, pipeline)) = pipelines_loaded_indices.pop() {
        pipeline_manager.processing_render_pipelines.remove(&id);
        pipeline_manager
            .loaded_render_pipelines
            .insert(id, pipeline);
        pipeline_manager
            .loaded_render_pipelines_desc
            .insert(id, pipelines_loaded_desc.remove(&id).unwrap());
    }

    // Update the shader to pipelines map
    pipeline_manager.shader_to_pipelines = shaders_to_pipelines;
}

/// Load the pipelines that are queued in the pipeline manager.
fn load_compute_pipelines(
    mut pipeline_manager: ResMut<PipelineManager>,
    render_instance: Res<RenderInstance>
) {
    let mut pipelines_loaded_indices: Vec<(usize, ComputePipeline)> = Vec::new();
    let mut pipelines_loaded_desc: HashMap<CachedPipelineIndex, ComputePipelineDescriptor> =
        HashMap::new();
    let mut shaders_to_pipelines: HashMap<AssetId<Shader>, Vec<CachedPipelineIndex>> =
        pipeline_manager.shader_to_pipelines.clone();
    for (id, descriptor) in pipeline_manager.processing_compute_pipelines.iter() {
        let mut can_load = true;

        // Check if compute shader is loaded
        let compute_shader = match &descriptor.comp {
            Some(shader) => {
                match pipeline_manager.shader_cache.get(&shader.id()) {
                    Some(shader) => Some(shader),
                    None => {
                        // Shader is not loaded yet
                        can_load = false;
                        None
                    }
                }
            }
            None => None
        };

        // Skip if shaders are not loaded
        if !can_load {
            continue;
        }
        pipelines_loaded_desc.insert(*id, descriptor.clone());
        shaders_to_pipelines
            .entry(descriptor.comp.as_ref().unwrap().id())
            .or_default()
            .push(*id);

        // Build the layouts
        let mut bind_group_layouts = Vec::new();
        for layout in descriptor.bind_group_layouts.iter() {
            let layout = match layout.as_ref().unwrap().build(&render_instance.0.read().unwrap()) {
                Ok(layout) => layout,
                Err(e) => {
                    error!(
                        "Failed to build bind group layout for pipeline {}: {:?}",
                        descriptor.label, e
                    );
                    continue;
                }
            };
            bind_group_layouts.push(layout);
        }

        // Load the pipeline
        let mut pipeline = ComputePipeline::new(descriptor.label);
        if let Some(compute_shader) = compute_shader {
            pipeline.set_shader(&compute_shader.content);
        }
        for push_constant in descriptor.push_constants.iter() {
            pipeline.add_push_constant(push_constant.offset, push_constant.size);
        }
        pipeline.set_bind_groups(bind_group_layouts);
        match pipeline.init(&render_instance.0.read().unwrap()) {
            Ok(_) => (),
            Err(e) => {
                error_once!("Failed to load pipeline: {:?}", e);
                continue;
            }
        }

        // Add the pipeline to the loaded pipelines
        pipelines_loaded_indices.push((*id, pipeline));
    }

    // Remove loaded pipelines and add them to the loaded pipelines
    while let Some((id, pipeline)) = pipelines_loaded_indices.pop() {
        pipeline_manager.processing_compute_pipelines.remove(&id);
        pipeline_manager
            .loaded_compute_pipelines
            .insert(id, pipeline);
        pipeline_manager
            .loaded_compute_pipelines_desc
            .insert(id, pipelines_loaded_desc.remove(&id).unwrap());
    }

    // Update the shader to pipelines map
    pipeline_manager.shader_to_pipelines = shaders_to_pipelines;
}
