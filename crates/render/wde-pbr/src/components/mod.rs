use bevy::prelude::{App, Plugin};

use crate::{components::lights::*, prelude::PbrModelRegistryPlugin};

pub mod lights;
pub mod model;
pub mod color;

pub(crate) struct PbrComponentsPlugin;
impl Plugin for PbrComponentsPlugin {
    fn build(&self, app: &mut App) {
        app
            .register_type::<DirectionalLight>()
            .register_type::<PointLight>()
            .register_type::<SpotLight>()
            .add_plugins(PbrModelRegistryPlugin);
    }
}
