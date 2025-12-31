use bevy::{platform::collections::HashMap, prelude::*};

/** Defines a render pass. */
pub trait RenderPass: Send + Sync {
    /**
     * Extract the pass elements from the main world to the render world.
     * Note: The main world is declared as mutable to allow for the extraction of resources,
     * however the modification of the main world will not be persisted.
     */
    fn extract(&self, _main_world: &mut World, _render_world: &mut World) {}

    /** Render the pass elements. */
    fn render(&self, _render_world: &mut World);
}

/** The index of a pass in the render graph. */
pub type PassIndex = u32;

/** A render graph. */
#[derive(Resource, Default)]
pub struct RenderGraph {
    passes: HashMap<PassIndex, Box<dyn RenderPass>>,
    sorted_passes: Vec<PassIndex>,
}
impl RenderGraph {
    /** 
     * Adds a new render pass to the render graph.
     * 
     * # Parameters
     * - `pass: P: RenderPass`: The pass to add.
     */
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

    /**
     * Extracts the render passes from the main world to the render world.
     * This method is automatically called by the render system.
     */
    pub(crate) fn extract(&mut self, main_world: &mut World, render_world: &mut World) {
        // Extract the passes
        for pass in self.sorted_passes.iter().map(|id| self.passes.get(id).unwrap()) {
            pass.extract(main_world, render_world);
        }
    }

    /**
     * Calls the render method for each pass in the render graph.
     * This method is automatically called by the render system.
     */
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
