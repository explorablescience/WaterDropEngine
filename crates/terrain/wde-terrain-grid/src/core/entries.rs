use wde_logger::prelude::*;

use bevy::prelude::*;
use wde_gltf::prelude::*;
use wde_scene::prelude::*;

const PLACEMENT_CONFIG_VERSION: u32 = 1;
const PLACEMENT_CONFIG_PATH: &str = "core/config/placement.json";

/// Entry for a single entity in the placement configuration.
#[derive(Clone, Debug, Reflect)]
pub struct PlacementConfigEntry {
    pub label: String,
    pub asset: Handle<GltfAsset>,
    pub extent: UVec2,
    pub anchors_in: Vec<Vec2>,
    pub anchors_out: Vec<Vec2>,
}

/// Configuration for the placement of entities in the terrain grid.
#[derive(Resource, Default)]
pub struct PlacementConfig {
    pending_entries: Vec<(String, Handle<GltfAsset>)>,
    pub labels: Vec<String>,
    pub entries: Vec<PlacementConfigEntry>,
}

pub struct PlacementPlugin;
impl Plugin for PlacementPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlacementConfig>()
            .add_systems(Startup, read_placement_file)
            .add_systems(Update, process_pending_entries);
    }
}

fn read_placement_file(
    asset_server: Res<AssetServer>,
    mut placement_config_struct: ResMut<PlacementConfig>,
) {
    // Load placement configuration file
    let placement_config = match serialize::parse_file(PLACEMENT_CONFIG_PATH) {
        Ok(config) => config,
        Err(err) => {
            error!(
                "Failed to load placement configuration file at '{}': {}",
                PLACEMENT_CONFIG_PATH, err
            );
            return;
        }
    };
    let placement_config = placement_config.data;

    // Check version
    if placement_config["version"].as_u64().unwrap_or(0) != PLACEMENT_CONFIG_VERSION as u64 {
        error!(
            "Unsupported placement configuration version: {}. Expected version: {}",
            placement_config["version"], PLACEMENT_CONFIG_VERSION
        );
        return;
    }

    // Load entities
    let config_entities = placement_config["entities"].as_array();
    let mut entries = Vec::new();
    for entity in config_entities.unwrap_or(&vec![]) {
        // Parse entity configuration
        let (label, model_path) = match (entity["label"].as_str(), entity["model"].as_str()) {
            (Some(label), Some(model_path)) => (label.to_string(), model_path.to_string()),
            _ => {
                error!(
                    "Invalid entity configuration in placement config: {:?}",
                    entity
                );
                continue;
            }
        };
        entries.push((label.clone(), asset_server.load(&model_path)));
    }
    placement_config_struct.pending_entries = entries;
}

fn process_pending_entries(
    mut placement_config_struct: ResMut<PlacementConfig>,
    gltf_assets: Res<Assets<GltfAsset>>,
) {
    if placement_config_struct.pending_entries.is_empty() {
        return;
    }
    let mut processed_entries = Vec::new();
    let mut processed_labels = Vec::new();
    let mut retry_entries = Vec::new();
    for (label, asset_handle) in placement_config_struct.pending_entries.drain(..) {
        if let Some(gltf_asset) = gltf_assets.get(&asset_handle) {
            let properties = &gltf_asset.properties;
            let extent = match properties.get("Footprint") {
                Some((_pos, scale)) => {
                    UVec2::new((scale.x).round() as u32, (scale.z).round() as u32)
                }
                None => {
                    error!(
                        "Extent not specified for label '{}', defaulting to (1, 1).",
                        label
                    );
                    UVec2::new(1, 1)
                }
            };
            let mut anchors_in = Vec::new();
            let mut anchors_out = Vec::new();
            for (anchor_name, (pos, _scale)) in properties.iter() {
                if anchor_name.starts_with("AnchorIn") {
                    anchors_in.push(Vec2::new(pos.x, pos.z));
                } else if anchor_name.starts_with("AnchorOut") {
                    anchors_out.push(Vec2::new(pos.x, pos.z));
                }
            }
            processed_entries.push(PlacementConfigEntry {
                label: label.clone(),
                asset: asset_handle.clone(),
                extent,
                anchors_in,
                anchors_out,
            });
            processed_labels.push(label);
        } else {
            retry_entries.push((label, asset_handle));
        }
    }
    placement_config_struct.entries.extend(processed_entries);
    placement_config_struct.labels.extend(processed_labels);
    placement_config_struct.pending_entries = retry_entries;
}
