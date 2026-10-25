#![allow(clippy::just_underscores_and_digits)]
#![allow(clippy::type_complexity)]

use bevy::prelude::*;
use wde::{CustomWdePlugins, prelude::*};

use crate::{test::TestPlugin, test_2::ColonistInfoPlugin};

mod test;
mod test_2;

pub fn main() {
    // Create the app
    let mut app = App::new();

    // Add default plugins
    app.add_plugins(WdeDefaultPlugins.set(CustomWdePlugins {
        window_icon:
            WindowIcon::from_bytes(include_bytes!("../../res/core/icons/app_icon.png")).ok(),
        ..Default::default()
    }))
    .add_plugins((TestPlugin, ColonistInfoPlugin));

    // Run the app
    app.run();
}
