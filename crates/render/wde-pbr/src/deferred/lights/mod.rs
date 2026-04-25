use bevy::prelude::*;

#[cfg(debug_assertions)]
mod editor;
mod lights_binding;
mod lights_types;
mod pbr_params;

pub use lights_binding::*;
pub use lights_types::*;
pub use pbr_params::*;

pub(crate) struct LightsPlugin;
impl Plugin for LightsPlugin {
    fn build(&self, app: &mut App) {
        // Register the lights
        app.register_type::<DirectionalLight>()
            .register_type::<PointLight>()
            .register_type::<SpotLight>();

        // Add the lights feature
        app.add_plugins(LightsBindingPlugin);

        #[cfg(debug_assertions)]
        app.add_plugins(editor::PbrLightingEditorPlugin);
    }
}
