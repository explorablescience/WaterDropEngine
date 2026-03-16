use bevy::prelude::*;

use crate::render::selected::object::SelectedObjectPassesPlugin;

mod object;
mod outline;

pub struct SelectedObjectPlugin;
impl Plugin for SelectedObjectPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(SelectedObjectPassesPlugin);
    }
}
