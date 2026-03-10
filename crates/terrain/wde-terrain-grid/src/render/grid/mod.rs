use bevy::prelude::*;

use crate::render::grid::cache::GridGizmoCache;

mod cache;
mod drawer;

pub(crate) struct GridRenderPlugin;
impl Plugin for GridRenderPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<GridGizmoCache>()
            .add_systems(Update, GridGizmoCache::setup_materials_and_meshes)
            .add_systems(Update, drawer::render_grid_bare)
            .add_systems(Update, drawer::render_grid_occupied_cells);
    }
}
