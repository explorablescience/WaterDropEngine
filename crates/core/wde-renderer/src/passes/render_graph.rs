use std::ops::Range;

use wde_logger::prelude::*;
use bevy::{platform::collections::HashMap, prelude::*};
use wde_wgpu::pipelines::BindGroup;
use crate::prelude::*;

use crate::{assets::{MeshAsset, Texture}, core::SwapchainFrame, prelude::CachedPipelineIndex};

pub use wde_wgpu::render_pass::RenderPassInstance;

/// Color attachment description for a render pass.
/// If `texture` is None, the pass will render to the swapchain texture.
pub struct RenderPassDescColorAttachment {
    /// The texture to render to.
    pub texture: AssetId<Texture>,
    /// How to load the texture at the start of the pass (default: Load).
    pub load: LoadOp<WgpuColor>,
    /// How to store the texture at the end of the pass (default: Store).
    pub store: StoreOp,
    /// Optional resolve target for multisampled textures. If set, the multisampled texture will be resolved into this texture at the end of the pass.
    pub resolve_target: Option<AssetId<Texture>>
}
impl Default for RenderPassDescColorAttachment {
    fn default() -> Self {
        Self {
            texture: AssetId::default(),
            load: LoadOp::Load,
            store: StoreOp::Store,
            resolve_target: None
        }
    }
}

/// Depth attachment description for a render pass.
pub struct RenderPassDescDepthAttachment {
    /// The texture to use as depth buffer.
    pub texture: Option<AssetId<Texture>>,
    /// How to load the texture at the start of the pass (default: Load).
    pub load: LoadOp<f32>,
    /// How to store the texture at the end of the pass (default: Store).
    pub store: StoreOp,
}
impl Default for RenderPassDescDepthAttachment {
    fn default() -> Self {
        Self {
            texture: None,
            load: LoadOp::Load,
            store: StoreOp::Store,
        }
    }
}

/// Descibes a render pass and its execution logic.
#[derive(Default)]
pub struct RenderPassDesc {
    /// Optional depth attachment. If None, no depth buffer is used.
    pub attachments_depth: Option<RenderPassDescDepthAttachment>,
    /// Optional color attachment. If None, the pass renders to the swapchain texture.
    pub attachments_colors: Option<Vec<RenderPassDescColorAttachment>>
}

/// A batch of draw commands that can be issued together with the same pipeline and bind groups.
pub struct DrawCommandsBatch {
    pub bind_group: Option<(u32, BindGroup)>, // index, bind group
    pub index_range: Range<u32>,
    pub instance_range: Range<u32>,
}

/// Commands to execute in a render pass, in order. For example: set pipeline, set bind groups, draw calls, etc.
pub enum SubPassCommand {
    /// Set the pipeline for subsequent draw calls.
    Pipeline(Option<CachedPipelineIndex>),
    /// Set a bind group at the given index for subsequent draw calls.
    BindGroup(u32, Option<BindGroup>),
    /// Set the vertex and index buffers for subsequent draw calls.
    Mesh(Option<AssetId<MeshAsset>>),
    /// Issue draw calls with the given index and instance ranges, using the currently set pipeline and bind groups.
    DrawBatches(Vec<DrawCommandsBatch>),
    /// Execute a custom function that has access to the render world and the render pass instance. This can be used for custom rendering logic that doesn't fit into the other command types. This function should be written as `fn _name_(world: &World, render_pass: &mut RenderPassInstance) { ... }`.
    Custom(for<'pass> fn(&'pass World, &mut RenderPassInstance<'pass>))
}

/// A sub-pass is a sequence of commands executed within a render pass.
/// For example, a GBuffer pass might have one sub-pass for rendering opaque objects and another for transparent objects.
#[derive(Default)]
pub struct SubPassDesc(pub Vec<SubPassCommand>);


/// Core trait for all render passes in the render graph.
///
/// A `RenderPass` has the (`render`) method. It runs in the render world. Issue GPU commands using the extracted data,
///    pull pipelines from the `PipelineManager`, use cached bind groups, and execute draw calls.
///
/// Both phases can be no-ops; for example, a simple pass might only implement `render`.
pub trait RenderPass: Send + Sync {
    /// Execute render commands for this pass.
    ///
    /// Called once per frame in the render world after all extracts complete.
    /// Access pipelines via `PipelineManager`, bind groups from resources,
    /// GPU meshes via `RenderAssets<GpuMesh>`, etc.
    ///
    /// This is where you issue actual draw calls (bind pipeline, bind groups, draw indexed, etc.).
    fn render(&self, _render_world: &mut World);
    
    /// Name of the pass, used for logging and debugging.
    fn label(&self) -> &str;


    fn process(&self, world: &World, pass_desc: &RenderPassDesc, sub_pass_desc: &SubPassDesc) {
        let label = self.label();
        let _span = debug_span!("render-pass-{}", label).entered();

        // Query render instance
        let render_instance = world.get_resource::<RenderInstance>().unwrap();
        let render_instance = render_instance.0.read().unwrap();

        // Get generic render assets handlers
        let textures = world.get_resource::<RenderAssets<GpuTexture>>().unwrap();
        let meshes = world.get_resource::<RenderAssets<GpuMesh>>().unwrap();
        let pipeline_manager = world.get_resource::<PipelineManager>().unwrap();

        // Handle pass
        let mut command_buffer = CommandBuffer::new(&render_instance, label);
        {
            let mut should_return = false;
            let mut render_pass =
                command_buffer.create_render_pass(label, |builder: &mut RenderPassBuilder| {
                    // Set color attachments
                    if pass_desc.attachments_colors.is_none() {
                        let swapchain_frame = world.get_resource::<SwapchainFrame>().unwrap().data.as_ref().unwrap();
                        builder.add_color_attachment(RenderPassColorAttachment {
                            texture: Some(&swapchain_frame.view),
                            ..default()
                        });
                    } else {
                        for color_attachment_desc in pass_desc.attachments_colors.as_ref().unwrap().iter() {
                            if let Some(texture) = textures.get(color_attachment_desc.texture)
                                && render_instance.surface_config.as_ref().unwrap().width == texture.texture.size.0
                                && render_instance.surface_config.as_ref().unwrap().height == texture.texture.size.1
                            {
                                let resolve_target = if let Some(resolve_target) = color_attachment_desc.resolve_target {
                                    if let Some(resolve_target) = textures.get(resolve_target)
                                        && render_instance.surface_config.as_ref().unwrap().width == resolve_target.texture.size.0
                                        && render_instance.surface_config.as_ref().unwrap().height == resolve_target.texture.size.1 {
                                        Some(resolve_target)
                                    } else {
                                        should_return = true;
                                        return;
                                    }
                                } else { None };
                                builder.add_color_attachment(RenderPassColorAttachment {
                                    texture: Some(&texture.texture.view),
                                    load: color_attachment_desc.load,
                                    store: color_attachment_desc.store,
                                    resolve_target: resolve_target.map(|tex| &tex.texture.view)
                                });
                            } else { should_return = true; }
                        }
                    }

                    // Set depth attachments
                    if pass_desc.attachments_depth.is_some() {
                        if let Some(depth_texture) = textures.get(pass_desc.attachments_depth.as_ref().unwrap().texture.unwrap())
                            && render_instance.surface_config.as_ref().unwrap().width == depth_texture.texture.size.0
                            && render_instance.surface_config.as_ref().unwrap().height == depth_texture.texture.size.1 {
                            builder.set_depth_texture(RenderPassDepth {
                                texture: Some(&depth_texture.texture.view),
                                load: pass_desc.attachments_depth.as_ref().unwrap().load,
                                store: pass_desc.attachments_depth.as_ref().unwrap().store
                            });
                        } else { should_return = true; }
                    }
                });
            if should_return {

                return;
            }

            // Issue global commands
            for stage_command in &sub_pass_desc.0 {
                match stage_command {
                    // Set pipeline
                    SubPassCommand::Pipeline(pipeline) => {
                        if let Some(pipeline) = pipeline
                            && let CachedPipelineStatus::OkRender(pipeline) = pipeline_manager.get_pipeline(*pipeline)
                        {
                            if let Err(e) = render_pass.set_pipeline(pipeline) {
                                error!("Failed to set pipeline: {:?}.", e);
                                return;
                            }
                        } else { return }
                    }

                    // Set bind groups at given index
                    SubPassCommand::BindGroup(index, bind_group) => {
                        if let Some(bind_group) = bind_group {
                            render_pass.set_bind_group(*index, bind_group);
                        } else { return }
                    }

                    // Bind mesh vertex and index buffers
                    SubPassCommand::Mesh(mesh) => {
                        if let Some(mesh) = mesh
                            && let Some(mesh) = meshes.get(*mesh)
                        {
                            render_pass.set_vertex_buffer(0, mesh.vertex_buffer.as_ref().unwrap());
                            render_pass.set_index_buffer(mesh.index_buffer.as_ref().unwrap());
                        } else { return }
                    }

                    // Issue draw calls
                    SubPassCommand::DrawBatches(batches) => {
                        for batch in batches {
                            // Set bind group
                            if let Some((index, bind_group)) = &batch.bind_group {
                                render_pass.set_bind_group(*index, bind_group);
                            }

                            // Draw the mesh
                            match render_pass.draw_indexed(batch.index_range.clone(), batch.instance_range.clone()) {
                                Ok(_) => {}
                                Err(e) => {
                                    error!("Failed to draw: {:?}.", e);
                                }
                            }
                        }
                    }

                    // Custom commands
                    SubPassCommand::Custom(custom_fn) => {
                        custom_fn(world, &mut render_pass);
                    }
                }
            }
        }
        command_buffer.submit(&render_instance);
    }
}

/// Unique identifier for a render pass in the graph.
pub type PassIndex = u32;

/// Manages ordered execution of render passes.
///
/// Stores instances of all registered passes and calls their extract/render methods
/// in numeric ID order each frame. Lower IDs run first, so dependencies can be expressed
/// through ordering (e.g., gbuffer at ID 0, lighting at ID 10).
#[derive(Resource, Default)]
pub struct RenderGraph {
    passes: HashMap<PassIndex, Box<dyn RenderPass>>,
    sorted_passes: Vec<PassIndex>,
}
impl RenderGraph {
    /// Register a new pass in the render graph.
    ///
    /// The pass runs in numeric ID order; lower IDs execute first.
    /// If a pass with this ID already exists, logs an error and does nothing.
    ///
    /// # Arguments
    /// - `id`: Numeric identifier; controls execution order.
    pub fn add_pass<P: RenderPass + 'static + Default>(&mut self, id: u32) {
        // Create pass
        let pass = P::default();

        // Test if the pass already exists
        if self.passes.contains_key(&id) {
            error!("The pass with id {} (with name {}) already exists in the render graph.", id, pass.label());
            return;
        }
        debug!("Adding the render pass {} at index {} to the render graph.", pass.label(), id);

        // Add the pass
        self.passes.insert(id, Box::new(pass));

        // Sort the passes
        self.sorted_passes = self.passes.keys().copied().collect();
        self.sorted_passes.sort();
    }

    /// Render phase: issue GPU commands for each pass.
    /// (Internal; called automatically by the renderer.)
    pub(crate) fn render(render_world: &mut World) {
        // Check if there is a swapchain frame
        if !render_world.contains_resource::<SwapchainFrame>() {
            warn!("No swapchain frame available, skipping render passes.");
            return;
        }
        let swapchain_frame = render_world.resource::<SwapchainFrame>();
        if swapchain_frame.data.is_none() {
            warn!("No swapchain texture view available, skipping render passes.");
            return;
        }

        // Run the update methods for each pass
        render_world.resource_scope(|render_world, graph: Mut<RenderGraph>| {
            for id in graph.sorted_passes.iter() {
                let pass = graph.passes.get(id).unwrap();
                let _span = debug_span!("render_pass_render", pass_id = *id, pass_name = pass.label()).entered();
                pass.render(render_world);
            }
        });
    }

    pub fn get_pass(&self, id: &PassIndex) -> Option<&dyn RenderPass> {
        self.passes.get(id).map(|boxed| boxed.as_ref())
    }
    pub fn get_sorted_passes(&self) -> &Vec<PassIndex> {
        &self.sorted_passes
    }
}

