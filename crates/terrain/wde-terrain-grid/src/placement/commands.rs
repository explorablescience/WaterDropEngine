//! Drives the placement tools from a game UI, alongside the editor window.
use bevy::prelude::*;

use crate::{
    core::entries::PlacementConfig,
    placement::{
        TerrainPlacementManager, TerrainPlacementMode, move_and_delete::MoveAndDeleteManager,
        ui::reset_tool
    }
};

/// Selects a placement tool.
#[derive(Message, Clone, Debug)]
pub enum PlacementCommand {
    /// Place the building with this config label.
    Build(String),
    /// Draw a network (e.g. `path`) with the line tool.
    DrawNetwork(String),
    /// Pick placed buildings up again.
    Move,
    /// Demolish placed buildings.
    Remove,
    /// Drop the current tool.
    Cancel
}

/// The active placement tool, for game UIs to highlight it.
#[derive(Resource, Default, Clone, PartialEq, Eq, Debug)]
pub enum PlacementTool {
    #[default]
    None,
    /// Placing the building with this config label.
    Build(String),
    /// Drawing this network.
    DrawNetwork(String),
    Move,
    Remove
}

pub(crate) fn apply_placement_commands(
    mut commands: Commands,
    mut messages: MessageReader<PlacementCommand>,
    mut manager: ResMut<TerrainPlacementManager>,
    mut move_and_delete_manager: ResMut<MoveAndDeleteManager>,
    config: Res<PlacementConfig>
) {
    for command in messages.read() {
        reset_tool(&mut commands, &mut manager, &mut move_and_delete_manager);
        manager.mode = match command {
            PlacementCommand::Build(label) => {
                manager.new_entry_to_place =
                    config.entries.iter().find(|e| &e.label == label).cloned();
                TerrainPlacementMode::Place
            }
            PlacementCommand::DrawNetwork(network) => {
                manager.line_network = Some(network.clone());
                TerrainPlacementMode::Place
            }
            PlacementCommand::Move => TerrainPlacementMode::Move,
            PlacementCommand::Remove => TerrainPlacementMode::Remove,
            PlacementCommand::Cancel => TerrainPlacementMode::None
        };
    }
}

pub(crate) fn update_placement_tool(
    manager: Res<TerrainPlacementManager>,
    mut tool: ResMut<PlacementTool>
) {
    let current = match manager.mode {
        TerrainPlacementMode::Place => {
            match (&manager.place_selected_entry_label, &manager.line_network) {
                (Some(label), _) => PlacementTool::Build(label.clone()),
                (None, Some(network)) => PlacementTool::DrawNetwork(network.clone()),
                (None, None) => PlacementTool::None
            }
        }
        TerrainPlacementMode::Move => PlacementTool::Move,
        TerrainPlacementMode::Remove => PlacementTool::Remove,
        TerrainPlacementMode::None => PlacementTool::None
    };
    tool.set_if_neq(current);
}
