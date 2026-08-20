use bevy::prelude::*;

mod navgrid;
mod navigation;

pub mod prelude {
    pub use super::TerrainNavigator;
}

pub struct TerrainNavigationPlugin;
impl Plugin for TerrainNavigationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((navigation::NavigationPlugin, navgrid::NavMapPlugin))
            .init_resource::<TerrainNavigator>()
            .add_systems(Update, handle_changes);
    }
}

#[derive(Resource, Default)]
pub struct TerrainNavigator {
    new_entities: Vec<(Entity, Vec3)>,
    removed_entities: Vec<Entity>
}
impl TerrainNavigator {
    pub fn add(&mut self, entity: Entity, position: Vec3) {
        self.new_entities.push((entity, position));
    }

    pub fn remove(&mut self, entity: Entity) {
        self.removed_entities.push(entity);
    }
}

#[derive(Component)]
pub(crate) struct TargetLocation(Vec3);

fn handle_changes(mut commands: Commands, mut navigator: ResMut<TerrainNavigator>) {
    for (entity, position) in navigator.new_entities.drain(..) {
        commands.entity(entity).insert(TargetLocation(position));
    }

    for entity in navigator.removed_entities.drain(..) {
        commands.entity(entity).remove::<TargetLocation>();
    }
}
