use bevy::prelude::*;

use crate::ssbos::ssbo_mesh::SsboMeshPlugin;

pub mod ssbo_mesh;

#[derive(Resource)]
pub(crate) struct SsboPlugin;
impl Plugin for SsboPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(SsboMeshPlugin);
    }
}
