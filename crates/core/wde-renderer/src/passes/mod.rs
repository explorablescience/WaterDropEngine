use bevy::prelude::*;

use crate::{core::{Extract, RenderApp}, passes::post_process_mesh::PostProcessingMesh};

pub mod depth;
pub mod render_graph;
pub mod post_process_mesh;

pub(crate) struct RendererPlugin;
impl Plugin for RendererPlugin {
    fn build(&self, app: &mut App) {
        // Add the depth rendering plugin
        app.add_plugins(depth::RendererPlugin);

        // Add the creation of the mesh for post-processing passes
        app
            .init_resource::<PostProcessingMesh>()
            .add_systems(Startup, PostProcessingMesh::init);
        app.get_sub_app_mut(RenderApp).unwrap()
            .init_resource::<PostProcessingMesh>()
            .add_systems(Extract, PostProcessingMesh::extract);
    }
}
