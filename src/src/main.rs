#![allow(clippy::just_underscores_and_digits)]
#![allow(clippy::type_complexity)]

use bevy::prelude::*;
use wde::prelude::*;

use crate::test::TestPlugin;

mod test;

pub fn main() {
    // Create the app
    let mut app = App::new();

    // Add default plugins
    app.add_plugins(WdeDefaultPlugins).add_plugins(TestPlugin);

    // Run the app
    app.run();
}
