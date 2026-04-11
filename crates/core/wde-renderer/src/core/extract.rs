//! Main systems for extracting data from the main world into the render world.
use wde_logger::prelude::*;

use bevy::prelude::*;

use crate::sync::sync_worlds;

use super::{EmptyWorld, Extract, MainWorld};

/// The extract system for the renderer.
/// This system is responsible for moving the main world into the render world.
/// Then, it runs the extract schedule.
/// Extract commands are registered during the extract schedule but are not applied until the apply_extract_commands system is run.
pub(crate) fn main_extract(main_world: &mut World, render_world: &mut World) {
    // Sync entities while the real main world is still available.
    {
        let _span = debug_span!("sync").entered();
        sync_worlds(main_world, render_world);
    }

    {
        let _span = debug_span!("extract").entered();
        // Temporarily add the main world to the render world
        let empty_world = main_world.remove_resource::<EmptyWorld>().unwrap();
        let previous_main_world = std::mem::replace(main_world, empty_world.0);
        render_world.insert_resource(MainWorld(previous_main_world));

        // Run the extract schedule
        render_world.run_schedule(Extract);

        // Move the app world back
        let inserted_world = render_world.remove_resource::<MainWorld>().unwrap();
        let empty_world = std::mem::replace(main_world, inserted_world.0);
        main_world.insert_resource(EmptyWorld(empty_world));
    }
}

/// Apply the extract commands registered during the extract schedule.
pub(crate) fn apply_extract_commands(render_world: &mut World) {
    render_world.resource_scope(|render_world, mut schedules: Mut<Schedules>| {
        let _apply_span = debug_span!("apply_extract_commands").entered();
        schedules
            .get_mut(Extract)
            .unwrap()
            .apply_deferred(render_world);
    });
}
