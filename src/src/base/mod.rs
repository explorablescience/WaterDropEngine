mod debug;
mod electricity;
mod properties;
mod sync;

use wde::prelude::*;

use bevy::prelude::*;

pub use electricity::BaseElectricity;
pub use properties::BuildingsConfig;

/// Identifies a base. It lives as long as the control room is present, and dies with it.
/// The members of a base can be found through the [`InBase`] relationship.
#[derive(Component, Debug)]
#[require(BaseElectricity)]
pub struct Base {
    pub control_room: Entity
}

/// Building or module working in a [`Base`].
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
#[relationship(relationship_target = BaseMembers)]
pub struct InBase(pub Entity);

/// Buildings and modules of a [`Base`], control room included.
#[derive(Component, Debug)]
#[relationship_target(relationship = InBase)]
pub struct BaseMembers(Vec<Entity>);

pub struct BasePlugin;
impl Plugin for BasePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(debug::BaseDebugPlugin)
            .init_resource::<BuildingsConfig>()
            .add_systems(Startup, properties::read_buildings_file)
            .add_systems(
                Update,
                (
                    properties::insert_properties,
                    sync::update_bases.after(BuildingGraphSet),
                    electricity::update_electricity
                )
                    .chain()
            );
    }
}
