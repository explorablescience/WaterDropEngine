mod drag;
mod select;

use wde::prelude::*;

use bevy::prelude::*;

/// Describes the type of entity that is currently selected or hovered over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntitySelectionType {
    Building(Entity),
    Module(Entity),
    Entity(Entity),
}
impl EntitySelectionType {
    pub fn entity(&self) -> Entity {
        match self {
            EntitySelectionType::Building(entity) => *entity,
            EntitySelectionType::Module(entity) => *entity,
            EntitySelectionType::Entity(entity) => *entity,
        }
    }
}

/// Describes the current selection state of the game, including hovered and selected entities.
#[derive(Resource, Default)]
pub struct SelectionState {
    pub hovered: Option<EntitySelectionType>,
    pub selected: Vec<EntitySelectionType>
}

/// Describes the materials used for outlining selected and hovered entities.
#[derive(Resource, Default)]
pub struct SelectionOutlineMaterial {
    hovered: Handle<OutlineMaterial>,
    selected: Handle<OutlineMaterial>
}
impl SelectionOutlineMaterial {
    /// Outline of `entity` in the given selection state, selection taking precedence over hover.
    fn get(&self, state: &SelectionState, entity: Entity) -> Option<&Handle<OutlineMaterial>> {
        if state.selected.iter().any(|selected| selected.entity() == entity) {
            Some(&self.selected)
        } else if state.hovered.is_some_and(|hovered| hovered.entity() == entity) {
            Some(&self.hovered)
        } else {
            None
        }
    }
}

pub struct SelectionPlugin;
impl Plugin for SelectionPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<SelectionState>()
            .init_resource::<SelectionOutlineMaterial>()
            .add_plugins((select::SelectPlugin, drag::DragPlugin))
            .add_systems(Startup, init_outline_materials);
    }
}

fn init_outline_materials(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    commands.insert_resource(SelectionOutlineMaterial {
        selected: asset_server.add(OutlineMaterial {
            label: "selection-outline".to_string(),
            color: WdeColor::from_srgba(0.3, 0.95, 0.4, 1.0),
            // ~3 px at 1080p with a 60° FOV, as thickness is a fraction of the view depth
            thickness: 0.003,
            ..Default::default()
        }),
        hovered: asset_server.add(OutlineMaterial {
            label: "hover-outline".to_string(),
            color: WdeColor::from_srgba(1.0, 1.0, 1.0, 0.6),
            thickness: 0.002,
            ..Default::default()
        })
    });
}
