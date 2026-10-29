mod animate;
mod spawn;

use wde::prelude::*;

use bevy::prelude::*;

/// Collider group for colonists and other entities that can interact with the terrain.
pub const ENTITIES_COLLIDER_GROUP: ColliderGroup = ColliderGroup::GROUP_4;

/// A marker that indicates a colonist entity. Each colonist is culled as a single group.
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(CullGroup)]
pub struct ColonistMarker;

/// A message to spawn a colonist in the world.
#[derive(Message, Clone, Reflect)]
pub struct MSpawnColonist {
    pub name: String,
    pub position: Vec3
}

pub struct ColonistPlugin;
impl Plugin for ColonistPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<MSpawnColonist>()
            .register_type::<ColonistMarker>()
            .register_type::<MSpawnColonist>()
            .add_plugins((animate::AnimateColonistPlugin, spawn::SpawnColonistPlugin));
    }
}
