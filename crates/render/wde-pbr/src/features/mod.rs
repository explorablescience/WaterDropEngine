use bevy::prelude::*;

use crate::features::lights::LightsFeature;

pub mod lights;

pub(crate) struct PbrFeaturesPlugin;
impl Plugin for PbrFeaturesPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(LightsFeature);
    }
}
