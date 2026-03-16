use bevy::prelude::*;

use crate::render::selected::SelectedObjectPlugin;

pub mod grid;
pub mod selected;

pub(crate) struct RenderPlugin;
impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(grid::GridRenderPlugin)
            .add_plugins(SelectedObjectPlugin);
    }
}
