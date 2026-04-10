use bevy::prelude::*;
use wde_renderer::prelude::*;

use crate::
    prelude::PbrMaterial
;

/// Store the render batches for PBR models
#[derive(Resource, Default, Debug)]
pub(crate) struct Batches {
    /// The render batches
    pub render_batches: Vec<Batch>,
    /// Pointers to the transform IDs in the SSBO for each instance in the batches
    pub transform_ids: Vec<u32>
}

/// A single render batch
#[derive(Debug, Default)]
pub(crate) struct Batch {
    pub mesh_id: AssetId<Mesh>,
    pub material_id: AssetId<PbrMaterial>,
    pub first_instance: u32,
    pub instance_count: u32
}

pub(crate) struct BatchesPlugin;
impl Plugin for BatchesPlugin {
    fn build(&self, app: &mut App) {
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .init_resource::<Batches>();
    }
}
