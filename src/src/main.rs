#![allow(clippy::just_underscores_and_digits)]
#![allow(clippy::type_complexity)]

use wde::prelude::*;
use bevy::prelude::*;

use crate::test::TestPlugin;

mod test;

pub fn main() {
    // Create the app
    let mut app = App::new();

    // Add default plugins
    info!("Adding default plugins.");
    app
        .add_plugins(WdeDefaultPlugins)
        .add_plugins(TestPlugin);

    // Run the app
    info!("Running game engine.");
    app.run();
}
