use bevy::prelude::*;

mod lights_binding;
mod lights_types;

pub use lights_binding::*;
pub use lights_types::*;

pub(crate) struct LightsPlugin;
impl Plugin for LightsPlugin {
    fn build(&self, app: &mut App) {
        // Register the lights
        app.register_type::<DirectionalLight>()
            .register_type::<PointLight>()
            .register_type::<SpotLight>();

        // Add the lights feature
        app.add_plugins(LightsBindingPlugin);
    }
}
