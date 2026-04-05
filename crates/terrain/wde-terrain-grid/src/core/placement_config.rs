use bevy::prelude::*;
use wde_gltf::prelude::*;
use wde_logger::prelude::*;
use wde_scene::prelude::*;

const PLACEMENT_CONFIG_VERSION: u32 = 1;
const PLACEMENT_CONFIG_PATH: &str = "core/config/placement.json";

/// Entry for a single entity in the placement configuration.
#[derive(Clone)]
pub struct PlacementConfigEntry {
    pub label: String,
    pub asset: GltfAsset,
    pub extent: UVec2
}

/// Configuration for the placement of entities in the terrain grid.
#[derive(Resource, Default)]
pub struct PlacementConfig {
    pub labels: Vec<String>,
    pub entries: Vec<PlacementConfigEntry>
}

pub struct PlacementPlugin;
impl Plugin for PlacementPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlacementConfig>()
            .add_systems(Startup, init_placement);
    }
}

fn init_placement(
    asset_server: Res<AssetServer>,
    mut placement_config_struct: ResMut<PlacementConfig>
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
    let mut entities = Vec::new();
    let mut labels = Vec::new();
    for entity in config_entities.unwrap_or(&vec![]) {
        // Parse entity configuration
        let (label, model_path, extent) = match (
            entity["label"].as_str(),
            entity["model"].as_str(),
            entity["extent"]["x"].as_u64(),
            entity["extent"]["y"].as_u64()
        ) {
            (Some(label), Some(model_path), Some(extent_x), Some(extent_y)) => (
                label.to_string(),
                model_path.to_string(),
                UVec2::new(extent_x as u32, extent_y as u32)
            ),
            _ => {
                error!(
                    "Invalid entity configuration in placement config: {:?}",
                    entity
                );
                continue;
            }
        };

        // Load model
        let gltf_model = match GltfLoader::load(&model_path, &asset_server) {
            Ok(model) => model,
            Err(err) => {
                error!("Failed to load model for entity '{}': {}", label, err);
                continue;
            }
        };

        entities.push(PlacementConfigEntry {
            label: label.clone(),
            asset: gltf_model,
            extent
        });
        labels.push(label);
    }

    // Store in resource
    placement_config_struct.labels = labels;
    placement_config_struct.entries = entities;
}
