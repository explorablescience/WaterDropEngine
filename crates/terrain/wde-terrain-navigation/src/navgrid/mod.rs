use bevy::prelude::*;

mod debug;
mod exterior;
mod interior;

pub struct NavMapPlugin;
impl Plugin for NavMapPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            interior::NavMapInteriorPlugin,
            debug::NavMapDebugPlugin,
            exterior::NavMapExteriorPlugin,
        ))
        .init_resource::<NavMap>();
    }
}

#[derive(Resource, Default)]
struct NavMap {
    interior: interior::InteriorNavMap,
    exterior: exterior::ExteriorNavMap,
}
