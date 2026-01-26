#![allow(clippy::just_underscores_and_digits)]
#![allow(clippy::type_complexity)]

use wde::prelude::*;
use bevy::prelude::*;

pub fn main() {
    // Create the app
    let mut app = App::new();

    // Add default plugins
    info!("Adding default plugins.");
    app.add_plugins(WdeDefaultPlugins);
    app.add_plugins(EguiPlugin);

    // Run the app
    info!("Running game engine.");
    app.run();
}
