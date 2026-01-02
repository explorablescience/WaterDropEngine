use bevy::{platform::collections::HashMap, prelude::*};

/// Core trait for all render passes in the render graph.
///
/// A `RenderPass` has two phases:
/// 1. **Extract** (`extract`): Runs in the main world context. Copy camera, mesh, material,
///    and other relevant data from the main app into the render world.
/// 2. **Render** (`render`): Runs in the render world. Issue GPU commands using the extracted data,
///    pull pipelines from the `PipelineManager`, use cached bind groups, and execute draw calls.
///
/// Both phases can be no-ops; for example, a simple pass might only implement `render`.
pub trait RenderPass: Send + Sync {
    /// Extract data from the main world into the render world.
    ///
    /// Called once per frame during the extract schedule. You have mutable access to both
    /// worlds but changes to `main_world` are not persisted (only the render world state matters).
    ///
    /// Use Bevy `SystemState` or direct world queries to pull data from main_world, then
    /// insert resources or components into render_world.
    ///
    /// # Example
    /// ```ignore
    /// fn extract(&self, main_world: &mut World, render_world: &mut World) {
    ///     let mut state = SystemState::<Query<(&Camera, &Transform)>>::new(main_world);
    ///     let cameras = state.get(main_world);
    ///     // ... copy camera data to render_world ...
    /// }
    /// ```
    fn extract(&self, _main_world: &mut World, _render_world: &mut World) {}

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

    /// Extract phase: copy data from main world to render world.
    /// (Internal; called automatically by the renderer.)
    pub(crate) fn extract(&mut self, main_world: &mut World, render_world: &mut World) {
        // Extract the passes
        for pass in self.sorted_passes.iter().map(|id| self.passes.get(id).unwrap()) {
            pass.extract(main_world, render_world);
        }
    }

    /// Render phase: issue GPU commands for each pass.
    /// (Internal; called automatically by the renderer.)
    pub(crate) fn render(render_world: &mut World) {
        trace!("Rendering the render passes.");

        // Run the update methods for each pass
        render_world.resource_scope(|render_world, graph: Mut<RenderGraph>| {
            for pass in graph.sorted_passes.iter().map(|id| graph.passes.get(id).unwrap()) {
                pass.render(render_world);
            }
        });
    }
}

