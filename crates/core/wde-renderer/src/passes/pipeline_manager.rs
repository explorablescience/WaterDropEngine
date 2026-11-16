use super::{
    ComputePipelineDescriptor, DebugPipelineReport, DebugPipelineStatus, DebugShaderReport,
    DebugShaderStatus, RenderPipelineDescriptor
};
use crate::assets::PrepareAssetError;
use crate::core::RenderInstance;
use crate::{
    assets::Shader,
    core::{Render, RenderSet},
    sync::{Extract, ExtractSchedule}
};
use bevy::{
    app::{App, Plugin},
    asset::{AssetEvent, AssetId, AssetServer, Assets, Handle, LoadState},
    ecs::prelude::*,
    prelude::MessageReader
};
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use wde_logger::prelude::*;
use wde_wgpu::{
    RenderInstanceData,
    bind_group::BindGroupLayout,
    compute_pipeline::ComputePipeline,
    render_pipeline::{RenderPipeline, ShaderStages}
};

pub(crate) struct PipelineManagerPlugin;
impl Plugin for PipelineManagerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PipelineManager>()
            .add_systems(ExtractSchedule, extract_shaders)
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
    /// The pipeline failed to build, or doesn't exist. It is rebuilt when one of its shaders changes.
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

    /// Errors of the queued pipelines that failed to build, which wait for a change of their shaders.
    errors: HashMap<CachedPipelineIndex, String>,
    /// Labels of the pipelines not queued yet, waiting for their bind group layouts.
    waiting_layouts: HashSet<&'static str>,
    /// Pipelines requested while not ready, since they were last taken.
    misses: Mutex<Vec<CachedPipelineIndex>>,

    /// CPU-side cache of shader assets keyed by asset id.
    shader_cache: HashMap<AssetId<Shader>, Shader>,
    /// Mapping from shader asset ids to pipelines that reference them (for hot-reload).
    shader_to_pipelines: HashMap<AssetId<Shader>, Vec<CachedPipelineIndex>>,
    /// Shaders of pipelines whose creation is retried, kept alive so they are not reloaded on each try.
    retried_shaders: HashMap<AssetId<Shader>, Handle<Shader>>
}
impl PipelineManager {
    /// Push the creation of a render pipeline to the pipeline manager queue.
    pub fn create_render_pipeline<E: Send + Sync + 'static>(
        &mut self,
        descriptor: RenderPipelineDescriptor,
        asset: E
    ) -> Result<CachedPipelineIndex, PrepareAssetError<E>> {
        // Check that every bind group layout is Some
        if descriptor.bind_group_layouts.iter().any(Option::is_none) {
            self.retain_shaders([&descriptor.vert, &descriptor.frag]);
            self.waiting_layouts.insert(descriptor.label);
            return Err(PrepareAssetError::RetryNextUpdate(asset));
        }

        // Store the pipeline descriptor to the queued pipelines, which now keeps its shaders alive
        self.release_shaders([&descriptor.vert, &descriptor.frag]);
        self.waiting_layouts.remove(descriptor.label);
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
        if descriptor.bind_group_layouts.iter().any(Option::is_none) {
            self.retain_shaders([&descriptor.comp]);
            self.waiting_layouts.insert(descriptor.label);
            return Err(PrepareAssetError::RetryNextUpdate(asset));
        }

        // Store the pipeline descriptor to the queued pipelines, which now keeps its shader alive
        self.release_shaders([&descriptor.comp]);
        self.waiting_layouts.remove(descriptor.label);
        let id = self.pipeline_iter;
        self.processing_compute_pipelines.insert(id, descriptor);
        self.pipeline_iter += 1;
        Ok(id)
    }

    fn retain_shaders<'a>(
        &mut self,
        shaders: impl IntoIterator<Item = &'a Option<Handle<Shader>>>
    ) {
        for shader in shaders.into_iter().flatten() {
            self.retried_shaders
                .entry(shader.id())
                .or_insert_with(|| shader.clone());
        }
    }

    fn release_shaders<'a>(
        &mut self,
        shaders: impl IntoIterator<Item = &'a Option<Handle<Shader>>>
    ) {
        for shader in shaders.into_iter().flatten() {
            self.retried_shaders.remove(&shader.id());
        }
    }

    /// Get the status of a pipeline from its cached index.
    /// Returns loading/ready/error state for both render and compute pipelines.
    pub fn get_pipeline(&'_ self, id: CachedPipelineIndex) -> CachedPipelineStatus<'_> {
        let status = if self.errors.contains_key(&id) {
            CachedPipelineStatus::Error
        } else if self.processing_render_pipelines.contains_key(&id)
            || self.processing_compute_pipelines.contains_key(&id)
        {
            CachedPipelineStatus::Loading
        } else if let Some(pipeline) = self.loaded_render_pipelines.get(&id) {
            return CachedPipelineStatus::OkRender(pipeline);
        } else if let Some(pipeline) = self.loaded_compute_pipelines.get(&id) {
            return CachedPipelineStatus::OkCompute(pipeline);
        } else {
            error_once!("Pipeline with id {} not found", id);
            CachedPipelineStatus::Error
        };
        let mut misses = self.misses.lock().unwrap();
        if !misses.contains(&id) {
            misses.push(id);
        }
        status
    }

    /// Pipelines requested while not ready since the last call.
    pub(crate) fn take_misses(&self) -> Vec<CachedPipelineIndex> {
        std::mem::take(&mut self.misses.lock().unwrap())
    }

    /// Status of every pipeline, with their shaders.
    pub(crate) fn report(&self, asset_server: &AssetServer) -> Vec<DebugPipelineReport> {
        let shader = |handle: &Handle<Shader>| DebugShaderReport {
            path: handle
                .path()
                .map_or_else(|| format!("{:?}", handle.id()), |p| p.to_string()),
            status: if self.shader_cache.contains_key(&handle.id()) {
                DebugShaderStatus::Loaded
            } else if let Some(LoadState::Failed(e)) = asset_server.get_load_state(handle.id()) {
                DebugShaderStatus::Failed(e.to_string())
            } else {
                DebugShaderStatus::Loading
            }
        };
        let report =
            |id: &CachedPipelineIndex,
             label: &'static str,
             compute: bool,
             ready: bool,
             shaders: &[&Option<Handle<Shader>>]| DebugPipelineReport {
                id: Some(*id),
                label,
                compute,
                status: match self.errors.get(id) {
                    Some(error) => DebugPipelineStatus::Failed(error.clone()),
                    None if ready => DebugPipelineStatus::Ready,
                    None => DebugPipelineStatus::Pending
                },
                shaders: shaders.iter().copied().flatten().map(shader).collect()
            };

        let render = |(id, d): (_, &RenderPipelineDescriptor), ready| {
            report(id, d.label, false, ready, &[&d.vert, &d.frag])
        };
        let compute = |(id, d): (_, &ComputePipelineDescriptor), ready| {
            report(id, d.label, true, ready, &[&d.comp])
        };
        let mut reports: Vec<_> = (self.processing_render_pipelines.iter())
            .map(|p| render(p, false))
            .chain(
                self.loaded_render_pipelines_desc
                    .iter()
                    .map(|p| render(p, true))
            )
            .chain(
                self.processing_compute_pipelines
                    .iter()
                    .map(|p| compute(p, false))
            )
            .chain(
                self.loaded_compute_pipelines_desc
                    .iter()
                    .map(|p| compute(p, true))
            )
            .chain(
                self.waiting_layouts
                    .iter()
                    .map(|label| DebugPipelineReport {
                        id: None,
                        label,
                        compute: false,
                        status: DebugPipelineStatus::WaitingLayouts,
                        shaders: vec![]
                    })
            )
            .collect();
        reports.sort_by(|a, b| {
            (a.status.rank(), a.label, a.id).cmp(&(b.status.rank(), b.label, b.id))
        });
        reports
    }
}

/// Extract the shaders from the asset server and store them in the pipeline manager.
fn extract_shaders(
    mut pipeline_manager: ResMut<PipelineManager>,
    shaders: Extract<Res<Assets<Shader>>>,
    mut shader_events: Extract<MessageReader<AssetEvent<Shader>>>
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

    // Rebuild the pipelines using the modified shaders, failed ones included
    let manager = &mut *pipeline_manager;
    for id in updated_ids {
        let Some(p_ids) = manager.shader_to_pipelines.get(&id) else {
            continue;
        };
        for p_id in p_ids {
            manager.errors.remove(p_id);
            if manager.loaded_render_pipelines.remove(p_id).is_some() {
                let desc = manager.loaded_render_pipelines_desc.remove(p_id).unwrap();
                manager.processing_render_pipelines.insert(*p_id, desc);
            }
            if manager.loaded_compute_pipelines.remove(p_id).is_some() {
                let desc = manager.loaded_compute_pipelines_desc.remove(p_id).unwrap();
                manager.processing_compute_pipelines.insert(*p_id, desc);
            }
        }
    }
}

/// Source of a shader, `None` while it is loading. A missing shader has an empty source.
fn shader_source<'a>(
    cache: &'a HashMap<AssetId<Shader>, Shader>,
    shader: &Option<Handle<Shader>>
) -> Option<&'a str> {
    match shader {
        Some(shader) => cache.get(&shader.id()).map(|s| s.content.as_str()),
        None => Some("")
    }
}

/// Registers the shaders of a pipeline, to rebuild it when they change.
fn register_shaders(
    shader_to_pipelines: &mut HashMap<AssetId<Shader>, Vec<CachedPipelineIndex>>,
    id: CachedPipelineIndex,
    shaders: &[&Option<Handle<Shader>>]
) {
    for shader in shaders.iter().copied().flatten() {
        let pipelines = shader_to_pipelines.entry(shader.id()).or_default();
        if !pipelines.contains(&id) {
            pipelines.push(id);
        }
    }
}

macro_rules! build_layouts {
    ($instance:expr, $layouts:expr) => {
        $layouts
            .iter()
            .flatten()
            .map(|layout: &BindGroupLayout| {
                layout
                    .build($instance)
                    .map_err(|e| format!("Failed to build a bind group layout: {e:?}"))
            })
            .collect::<Result<Vec<_>, _>>()?
    };
}

fn build_render_pipeline(
    instance: &RenderInstanceData,
    descriptor: &RenderPipelineDescriptor,
    vert: &str,
    frag: &str
) -> Result<RenderPipeline, String> {
    let mut pipeline = RenderPipeline::new(descriptor.label);
    pipeline.set_shader(vert, ShaderStages::VERTEX);
    pipeline.set_shader(frag, ShaderStages::FRAGMENT);
    pipeline.set_fragment_blend(descriptor.fragment_blend);
    pipeline.set_color_write_mask(descriptor.color_write);
    pipeline.set_topology(descriptor.topology);
    pipeline.set_cull_mode(descriptor.cull_mode);
    pipeline.set_depth(descriptor.depth.clone());
    pipeline.set_use_vertices_buffer(descriptor.vertex_buffer);
    if let Some(ref render_targets) = descriptor.render_targets {
        pipeline.set_render_targets(render_targets.clone());
    }
    pipeline.set_sample_count(descriptor.sample_count);
    for push_constant in descriptor.push_constants.iter() {
        pipeline.add_push_constant(push_constant.offset, push_constant.size);
    }
    pipeline.set_bind_groups(build_layouts!(instance, descriptor.bind_group_layouts));
    pipeline.init(instance).map_err(|e| e.to_string())?;
    Ok(pipeline)
}

fn build_compute_pipeline(
    instance: &RenderInstanceData,
    descriptor: &ComputePipelineDescriptor,
    comp: &str
) -> Result<ComputePipeline, String> {
    let mut pipeline = ComputePipeline::new(descriptor.label);
    pipeline.set_shader(comp);
    for push_constant in descriptor.push_constants.iter() {
        pipeline.add_push_constant(push_constant.offset, push_constant.size);
    }
    pipeline.set_bind_groups(build_layouts!(instance, descriptor.bind_group_layouts));
    pipeline.init(instance).map_err(|e| e.to_string())?;
    Ok(pipeline)
}

/// Builds the queued render pipelines whose shaders are loaded.
fn load_render_pipelines(
    mut pipeline_manager: ResMut<PipelineManager>,
    render_instance: Res<RenderInstance>
) {
    let instance = render_instance.0.read().unwrap();
    let manager = &mut *pipeline_manager;
    let mut loaded = Vec::new();
    for (id, descriptor) in manager.processing_render_pipelines.iter() {
        if manager.errors.contains_key(id) {
            continue;
        }
        let (Some(vert), Some(frag)) = (
            shader_source(&manager.shader_cache, &descriptor.vert),
            shader_source(&manager.shader_cache, &descriptor.frag)
        ) else {
            continue;
        };
        register_shaders(
            &mut manager.shader_to_pipelines,
            *id,
            &[&descriptor.vert, &descriptor.frag]
        );
        match build_render_pipeline(&instance, descriptor, vert, frag) {
            Ok(pipeline) => {
                trace!("Loaded pipeline {} with id {}.", descriptor.label, id);
                loaded.push((*id, pipeline));
            }
            Err(e) => {
                error!(
                    "Failed to build the render pipeline '{}': {}",
                    descriptor.label, e
                );
                manager.errors.insert(*id, e);
            }
        }
    }
    for (id, pipeline) in loaded {
        let desc = manager.processing_render_pipelines.remove(&id).unwrap();
        manager.loaded_render_pipelines.insert(id, pipeline);
        manager.loaded_render_pipelines_desc.insert(id, desc);
    }
}

/// Builds the queued compute pipelines whose shader is loaded.
fn load_compute_pipelines(
    mut pipeline_manager: ResMut<PipelineManager>,
    render_instance: Res<RenderInstance>
) {
    let instance = render_instance.0.read().unwrap();
    let manager = &mut *pipeline_manager;
    let mut loaded = Vec::new();
    for (id, descriptor) in manager.processing_compute_pipelines.iter() {
        if manager.errors.contains_key(id) {
            continue;
        }
        let Some(comp) = shader_source(&manager.shader_cache, &descriptor.comp) else {
            continue;
        };
        register_shaders(&mut manager.shader_to_pipelines, *id, &[&descriptor.comp]);
        match build_compute_pipeline(&instance, descriptor, comp) {
            Ok(pipeline) => {
                trace!("Loaded pipeline {} with id {}.", descriptor.label, id);
                loaded.push((*id, pipeline));
            }
            Err(e) => {
                error!(
                    "Failed to build the compute pipeline '{}': {}",
                    descriptor.label, e
                );
                manager.errors.insert(*id, e);
            }
        }
    }
    for (id, pipeline) in loaded {
        let desc = manager.processing_compute_pipelines.remove(&id).unwrap();
        manager.loaded_compute_pipelines.insert(id, pipeline);
        manager.loaded_compute_pipelines_desc.insert(id, desc);
    }
}
