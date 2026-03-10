use bevy::prelude::*;

pub mod grid;

pub(crate) struct RenderPlugin;
impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(grid::GridRenderPlugin);
    }
}
