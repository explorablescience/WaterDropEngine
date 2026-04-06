use bevy::prelude::*;

mod lights_buffer;
mod lights_components;

pub use lights_buffer::*;
pub use lights_components::*;

pub(crate) struct LightsPlugin;
impl Plugin for LightsPlugin {
    fn build(&self, app: &mut App) {
        // Register the lights
        app.register_type::<DirectionalLight>()
            .register_type::<PointLight>()
            .register_type::<SpotLight>();

        // Add the lights feature
        app.add_plugins(LightsFeature);
    }
}
