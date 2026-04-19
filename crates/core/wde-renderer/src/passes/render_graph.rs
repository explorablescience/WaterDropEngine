use crate::prelude::*;
use wde_logger::prelude::*;
use wde_wgpu::passes::{
    CommandBuffer, RenderPassBuilder, RenderPassColorAttachment, RenderPassDepth
};

use bevy::ecs::system::{ReadOnlySystemParam, SystemParamItem, SystemState};
use bevy::prelude::*;
use std::collections::HashMap;
use std::ops::Range;
use wde_wgpu::pipelines::BindGroup;

use crate::{
    assets::{Mesh, Texture},
    core::SwapchainFrame
};

pub use wde_wgpu::command_buffer::{LoadOp, Operations, StoreOp};
pub use wde_wgpu::render_pass::RenderPassInstance;

/// Color attachment description for a render pass.
/// If `texture` is None, the pass will render to the swapchain texture.
pub struct RenderPassDescColorAttachment {
    /// The texture to render to.
    pub texture: Option<AssetId<Texture>>,
    /// How to load the texture at the start of the pass (default: Load).
    pub load: LoadOp<crate::prelude::Color>,
    /// How to store the texture at the end of the pass (default: Store).
    pub store: StoreOp,
    /// Optional resolve target for multisampled textures. If set, the multisampled texture will be resolved into this texture at the end of the pass.
    pub resolve_target: Option<AssetId<Texture>>
}
impl Default for RenderPassDescColorAttachment {
    fn default() -> Self {
        Self {
            texture: None,
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
    /// How to load the stencil at the start of the pass (default: Load).
    pub stencil_load: LoadOp<u32>,
    /// How to store the stencil at the end of the pass (default: Store).
    pub stencil_store: StoreOp
}
impl Default for RenderPassDescDepthAttachment {
    fn default() -> Self {
        Self {
            texture: None,
            load: LoadOp::Load,
            store: StoreOp::Store,
            stencil_load: LoadOp::Load,
            stencil_store: StoreOp::Store
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
    pub instance_range: Range<u32>
}
impl Default for DrawCommandsBatch {
    fn default() -> Self {
        Self {
            bind_group: None,
            index_range: 0..0,
            instance_range: 0..1
        }
    }
}
/// Commands to execute in a render pass, in order. For example: set pipeline, set bind groups, draw calls, etc.
pub enum SubPassCommand {
    /// Set the pipeline for subsequent draw calls.
    Pipeline(Option<CachedPipelineIndex>),
    /// Set a bind group at the given index for subsequent draw calls.
    BindGroup(u32, Option<BindGroup>),
    /// Set the vertex and index buffers for subsequent draw calls.
    Mesh(Option<AssetId<Mesh>>),
    /// Set the stencil reference value used by stencil compare and write operations.
    StencilReference(u32),
    /// Issue draw calls with the given index and instance ranges, using the currently set pipeline and bind groups.
    DrawBatches(Vec<DrawCommandsBatch>),
    /// Execute a custom function that has access to the render world and the render pass instance. This can be used for custom rendering logic that doesn't fit into the other command types. This function should be written as `fn _name_(world: &World, render_pass: &mut RenderPassInstance) { ... }`.
    Custom(for<'pass> fn(&'pass World, &mut RenderPassInstance<'pass>))
}
/// A sub-pass is a sequence of commands executed within a render pass.
/// For example, a GBuffer pass might have one sub-pass for rendering opaque objects and another for transparent objects.
#[derive(Default)]
pub struct RenderSubPassDesc(pub Vec<SubPassCommand>);

/// Type alias for render pass IDs. These are numeric identifiers that control the execution order of passes in the render graph; lower IDs execute first.
pub type RenderPassId = i32;
/// Describes a render pass in the render graph, with its attachments, load ops, etc.
/// This is returned by the `describe` method of the `RenderPass` trait, which is called in the render world before rendering to get the pass description and attachments.
/// See [crate::passes] for more details and examples.
pub trait RenderPass {
    type Params: ReadOnlySystemParam;

    /// Describe the render pass (attachments, load ops, etc).
    fn describe(params: &SystemParamItem<Self::Params>) -> RenderPassDesc;
    /// Return the unique numeric ID of this render pass. Passes execute in order of their IDs (lower IDs execute first).
    fn id() -> RenderPassId;
    /// Return the (non-unique) label of the render pass, used for logging and debugging.
    fn label() -> &'static str;

    /// Custom rendering logic for this pass.
    /// This can be used for passes that require custom rendering logic that doesn't fit into the default render graph execution flow.
    ///
    /// Notes:
    /// - Returns None by default, meaning the pass will be rendered with the default render graph execution flow.
    /// - If it returns Some(true), it means the pass executed custom rendering logic and should be considered successfully rendered for this frame.
    /// - The provided command buffer is already begun and will be submitted at the end of the render graph execution, so the custom rendering logic can simply record commands to it.
    fn custom_render(_world: &mut World, _command_buffer: &mut CommandBuffer) -> Option<bool> {
        None
    }
}
// Utility traits for the render graph implementation. Not meant to be used by users of the render graph.
trait RenderPassNode: Send + Sync + 'static {
    fn describe(&mut self, world: &mut World) -> RenderPassDesc;
    fn label(&mut self) -> &'static str;
    fn custom_render(
        &mut self,
        world: &mut World,
        command_buffer: &mut CommandBuffer
    ) -> Option<bool>;
}
struct RenderPassHolder<P: RenderPass> {
    _phantom: std::marker::PhantomData<P>
}
impl<P> RenderPassNode for RenderPassHolder<P>
where
    P: RenderPass + Send + Sync + 'static
{
    fn describe(&mut self, world: &mut World) -> RenderPassDesc {
        let mut state: SystemState<P::Params> = SystemState::new(world);
        let params = state.get(world);
        P::describe(&params)
    }
    fn label(&mut self) -> &'static str {
        P::label()
    }
    fn custom_render(
        &mut self,
        world: &mut World,
        command_buffer: &mut CommandBuffer
    ) -> Option<bool> {
        P::custom_render(world, command_buffer)
    }
}

/// Core trait for all render sub-passes in the render graph. A sub-pass is a sequence of commands executed within a render pass.
/// For example, a GBuffer pass might have one sub-pass for rendering opaque objects and another for transparent objects.
/// The `describe` method defines the commands to execute in this sub-pass, which can be setting pipelines, bind groups, vertex buffers, draw calls, or even custom rendering logic.
/// See [crate::passes] for more details and examples.
pub trait RenderSubPass {
    type Params: ReadOnlySystemParam;

    /// Describe the sub-pass commands to execute within the render pass.
    fn describe(params: &SystemParamItem<Self::Params>) -> RenderSubPassDesc;
    /// Return the (non-unique) label of this sub-pass, used for logging and debugging.
    fn label() -> &'static str;
}
// Utility traits for the render graph implementation. Not meant to be used by users of the render graph.
trait RenderSubPassNode: Send + Sync + 'static {
    fn describe(&mut self, world: &mut World) -> RenderSubPassDesc;
    fn label(&mut self) -> &'static str;
}
struct RenderSubPassHolder<SP: RenderSubPass> {
    _phantom: std::marker::PhantomData<SP>
}
impl<SP> RenderSubPassNode for RenderSubPassHolder<SP>
where
    SP: RenderSubPass + Send + Sync + 'static
{
    fn describe(&mut self, world: &mut World) -> RenderSubPassDesc {
        let mut state: SystemState<SP::Params> = SystemState::new(world);
        let params = state.get(world);
        SP::describe(&params)
    }
    fn label(&mut self) -> &'static str {
        SP::label()
    }
}

/// The render graph resource, which stores all render passes and sub-passes, their descriptions, and their execution order.
/// This is the core of the render graph system, and is responsible for executing the render passes in the correct order with the correct attachments and commands.
///
/// To register a new render pass, use the `add_pass` method with a type that implements the `RenderPass` trait. To register a sub-pass to a render pass, use the `add_sub_pass` method with a type that implements the `RenderSubPass` trait and the parent render pass type.
#[derive(Resource, Default)]
pub struct RenderGraph {
    // Raw storage of passes and subpasses
    passes: Vec<Box<dyn RenderPassNode>>,
    sub_passes: Vec<Box<dyn RenderSubPassNode>>,

    // Ordering logic for the passes
    sorted_ids: Vec<RenderPassId>, // pass indices in `passes`, sorted by their numeric IDs

    // Indexing for execution
    render_passes_by_id: HashMap<RenderPassId, usize>, // pass id -> pass index in `passes`
    sub_passes_by_renderpass: HashMap<usize, Vec<usize>>  // render pass index -> sub-pass indices
}
impl RenderGraph {
    /// Add a render pass to the graph. Passes execute in order of their IDs (lower IDs execute first).
    /// The pass type must implement the `RenderPass` trait, which defines the pass's description and execution logic.
    pub fn add_pass<P: RenderPass + Send + Sync + 'static>(&mut self) -> &mut Self {
        let id = P::id();

        // Assert that the pass ID is unique
        debug_assert!(
            !self.render_passes_by_id.contains_key(&id),
            "A render pass with ID {} already exists in the render graph.",
            id
        );

        // Insert the new pass
        self.passes.push(Box::new(RenderPassHolder {
            _phantom: std::marker::PhantomData::<P>
        }));
        self.render_passes_by_id.insert(id, self.passes.len() - 1);
        self.sorted_ids.push(id);

        // Sort the passes by ID
        self.sorted_ids.sort();
        self
    }

    /// Add a sub-pass to a render pass in the graph. The sub-pass will be executed in the order it was added within its parent render pass.
    /// The sub-pass type must implement the `RenderSubPass` trait, which defines the sub-pass's commands and the render pass it belongs to.
    pub fn add_sub_pass<SP: RenderSubPass + Send + Sync + 'static, P: RenderPass>(
        &mut self
    ) -> &mut Self {
        self.sub_passes.push(Box::new(RenderSubPassHolder {
            _phantom: std::marker::PhantomData::<SP>
        }));
        let sub_pass_index = self.render_passes_by_id.get(&P::id()).copied();
        debug_assert!(
            sub_pass_index.is_some(),
            "Cannot add sub-pass '{}' to render pass '{}': no render pass with ID {} found in the render graph.",
            SP::label(),
            P::label(),
            P::id()
        );
        self.sub_passes_by_renderpass
            .entry(sub_pass_index.unwrap())
            .or_default()
            .push(self.sub_passes.len() - 1);
        self
    }

    pub(crate) fn render(world: &mut World) {
        world.resource_scope(|world, mut graph: Mut<RenderGraph>| {
            render(&mut graph, world);
        });
    }
}

fn render(graph: &mut RenderGraph, world: &mut World) {
    let _span = debug_span!("render_graph").entered();

    // Create command buffer
    let mut command_buffer = {
        let render_instance = world.get_resource::<RenderInstance>().unwrap();
        CommandBuffer::new(&render_instance.0.read().unwrap(), "render-graph")
    };

    // Execute passes in order of their IDs
    for pass_id in &graph.sorted_ids {
        // Get pass description
        let pass_index = *graph.render_passes_by_id.get(pass_id).unwrap();
        let pass = &mut graph.passes[pass_index];
        let pass_label = pass.label();
        let pass_desc = pass.describe(world);

        // Try custom rendering logic for this pass
        if let Some(custom_render_result) = pass.custom_render(world, &mut command_buffer) {
            // If it was Some(true or false), it means the pass used custom rendering logic.
            if !custom_render_result {
                debug!(
                    "Custom rendering for pass '{}' could not be completed. Skipping this pass for this frame.",
                    pass_label
                );
            }
            continue;
        }

        // Precompute sub-pass descriptions before creating the render pass to avoid borrow conflicts with the world
        let mut sub_passes = Vec::new();
        if let Some(sub_pass_indices) = graph.sub_passes_by_renderpass.get(&pass_index) {
            for sub_pass_index in sub_pass_indices {
                let sub_pass = &mut graph.sub_passes[*sub_pass_index];
                let sub_pass_label = sub_pass.label();
                let sub_pass_desc = sub_pass.describe(world);
                sub_passes.push((sub_pass_label, sub_pass_desc));
            }
        }

        // Create the render pass from its description
        let _pass_span = debug_span!("render_pass", pass_id = *pass_id, pass_label).entered();
        let mut render_pass =
            match create_render_pass(world, &mut command_buffer, pass_label, &pass_desc) {
                Ok(pass) => pass,
                Err(e) => {
                    debug!(
                        "Failed to create render pass: '{}'. Skipping this frame.",
                        e
                    );
                    continue;
                }
            };

        // Execute sub-passes in order of addition
        for (sub_pass_label, sub_pass_desc) in &sub_passes {
            let _sub_pass_span =
                debug_span!("render_sub_pass", sub_pass_label = *sub_pass_label).entered();
            render_sub_pass(world, &mut render_pass, sub_pass_desc);
        }
    }

    // Submit commands
    let render_instance = world.get_resource::<RenderInstance>().unwrap();
    command_buffer.submit(&render_instance.0.read().unwrap());
}

fn create_render_pass<'p>(
    world: &'p World,
    command_buffer: &'p mut CommandBuffer,
    label: &str,
    pass_desc: &RenderPassDesc
) -> Result<RenderPassInstance<'p>, String> {
    // Get generic handlers
    let render_instance = world.get_resource::<RenderInstance>().unwrap();
    let textures = world.get_resource::<RenderAssets<GpuTexture>>().unwrap();
    let swapchain_frame = world.get_resource::<SwapchainFrame>().unwrap();
    let (surface_width, surface_height) = {
        let render_instance = render_instance.0.read().unwrap();
        let config = render_instance.surface_config.as_ref().unwrap();
        (config.width, config.height)
    };

    // Create the render pass
    command_buffer.create_render_pass(label, |builder: &mut RenderPassBuilder| -> Result<(), String> {
        // Set color attachments
        if let Some(attachments_color) = &pass_desc.attachments_colors {
            // Set specified color attachments
            for color_attachment_desc in attachments_color.iter() {
                if let Some(texture_id) = color_attachment_desc.texture
                    && let Some(texture) = textures.get(texture_id) {
                    if !(surface_width == texture.texture.size.0 && surface_height == texture.texture.size.1) {
                        return Err(format!("Color attachment texture has invalid size: expected ({}, {}), got ({}, {})", surface_width, surface_height, texture.texture.size.0, texture.texture.size.1));
                    }
                    let resolve_target = if let Some(resolve_target) = color_attachment_desc.resolve_target {
                        if let Some(resolve_target) = textures.get(resolve_target)
                            && surface_width == resolve_target.texture.size.0 && surface_height == resolve_target.texture.size.1 {
                            Some(resolve_target)
                        } else {
                            return Err("Invalid resolve target for color attachment".to_string());
                        }
                    } else { None };
                    builder.add_color_attachment(RenderPassColorAttachment {
                        texture: Some(&texture.texture.view),
                        load: match color_attachment_desc.load {
                            LoadOp::Load => LoadOp::Load,
                            LoadOp::Clear(color) => LoadOp::Clear(color.into())
                        },
                        store: color_attachment_desc.store,
                        resolve_target: resolve_target.map(|tex| &tex.texture.view)
                    });
                } else {
                    return Err("Invalid texture for color attachment".to_string());
                }
            }
        } else {
            // Set swapchain as color attachment if no other color attachments are specified
            let swapchain_frame = match swapchain_frame.data.as_ref() {
                Some(frame) => frame,
                None => return Err("No swapchain frame available".to_string())
            };
            builder.add_color_attachment(RenderPassColorAttachment {
                texture: Some(&swapchain_frame.view),
                ..default()
            });
        }

        // Set depth attachments
        if let Some(depth_attachment) = pass_desc.attachments_depth.as_ref() {
            if let Some(depth_texture) = depth_attachment.texture
               && let Some(depth_texture) = textures.get(depth_texture) {
                if !(surface_width == depth_texture.texture.size.0 && surface_height == depth_texture.texture.size.1) {
                    return Err(format!("Depth attachment texture has invalid size: expected ({}, {}), got ({}, {}).", surface_width, surface_height, depth_texture.texture.size.0, depth_texture.texture.size.1));
                }
                builder.set_depth_texture(RenderPassDepth {
                    texture: Some(&depth_texture.texture.view),
                    load: depth_attachment.load,
                    store: depth_attachment.store,
                    stencil_load: depth_attachment.stencil_load,
                    stencil_store: depth_attachment.stencil_store
                });
            } else {
                return Err("Invalid texture for depth attachment".to_string());
            }
        }
        Ok(())
    })
}

fn render_sub_pass<'p>(
    world: &'p World,
    render_pass: &mut RenderPassInstance<'p>,
    sub_pass_desc: &'p RenderSubPassDesc
) {
    // Get generic handlers
    let pipeline_manager = world.get_resource::<PipelineManager>().unwrap();
    let meshes = world.get_resource::<RenderAssets<GpuMesh>>().unwrap();

    // Issue global commands
    for stage_command in &sub_pass_desc.0 {
        match stage_command {
            // Set pipeline
            SubPassCommand::Pipeline(pipeline) => {
                if let Some(pipeline) = pipeline
                    && let CachedPipelineStatus::OkRender(pipeline) =
                        pipeline_manager.get_pipeline(*pipeline)
                {
                    if let Err(e) = render_pass.set_pipeline(pipeline) {
                        error!("Failed to set pipeline: {:?}.", e);
                        return;
                    }
                } else {
                    return;
                }
            }

            // Set bind groups at given index
            SubPassCommand::BindGroup(index, bind_group) => {
                if let Some(bind_group) = bind_group {
                    render_pass.set_bind_group(*index, bind_group);
                } else {
                    return;
                }
            }

            // Bind mesh vertex and index buffers
            SubPassCommand::Mesh(mesh) => {
                if let Some(mesh) = mesh
                    && let Some(mesh) = meshes.get(*mesh)
                {
                    render_pass.set_vertex_buffer(0, mesh.vertex_buffer.as_ref().unwrap());
                    render_pass.set_index_buffer(mesh.index_buffer.as_ref().unwrap());
                } else {
                    return;
                }
            }

            // Set stencil reference value
            SubPassCommand::StencilReference(reference) => {
                render_pass.set_stencil_reference(*reference);
            }

            // Issue draw calls
            SubPassCommand::DrawBatches(batches) => {
                for batch in batches {
                    // Set bind group
                    if let Some((index, bind_group)) = &batch.bind_group {
                        render_pass.set_bind_group(*index, bind_group);
                    }

                    // Draw the mesh
                    match render_pass
                        .draw_indexed(batch.index_range.clone(), batch.instance_range.clone())
                    {
                        Ok(_) => {}
                        Err(e) => {
                            error!("Failed to draw: {:?}.", e);
                        }
                    }
                }
            }

            // Custom commands
            SubPassCommand::Custom(custom_fn) => {
                custom_fn(world, render_pass);
            }
        }
    }
}
