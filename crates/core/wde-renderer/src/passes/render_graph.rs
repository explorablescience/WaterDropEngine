use wde_logger::prelude::*;
use bevy::{platform::collections::HashMap, prelude::*};

use crate::core::SwapchainFrame;

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
        // Test if the pass already exists
        if self.passes.contains_key(&id) {
            error!("The pass with id {} already exists in the render graph.", id);
            return;
        }
        info!("Adding a new render pass with id {} to the render graph.", id);

        // Add the pass
        self.passes.insert(id, Box::new(P::default()));

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
                let _span = debug_span!("render_pass_render", pass_id = *id).entered();
                let pass = graph.passes.get(id).unwrap();
                pass.render(render_world);
            }
        });
    }
}

