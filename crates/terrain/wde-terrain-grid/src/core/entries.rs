use wde_logger::prelude::*;

use bevy::prelude::*;
use wde_editor::prelude::*;
use wde_gltf::prelude::*;
use wde_renderer::prelude::*;
use wde_scene::prelude::*;

const PLACEMENT_CONFIG_VERSION: u32 = 1;
const PLACEMENT_CONFIG_PATH: &str = "core/config/placement.json";

/// Entry for a single entity in the placement configuration.
#[derive(Clone, Debug, Reflect)]
pub struct PlacementConfigEntry {
    pub label: String,
    /// Menu category, in the order categories first appear in the config.
    pub category: String,
    pub description: String,
    /// Network (e.g. `tunnel`, `path`) this piece belongs to, drawn with the line tool.
    pub network: Option<NetworkPiece>,
    /// Gameplay role, e.g. `control_room` for the building defining a base.
    pub role: Option<String>,
    /// Menu icon, registered in [UITextures].
    pub icon: Option<Handle<Texture>>,
    pub asset: Handle<GltfAsset>,
    /// Optional cutaway variant (open top), shown when [BuildingCutaway](crate::prelude::BuildingCutaway) is enabled.
    pub asset_cut: Option<Handle<GltfAsset>>,
    pub extent: UVec2,
    /// Tunnel connection points (`%P%AnchorIn*` properties).
    pub anchors_in: Vec<PlacementAnchor>,
    /// Path connection points (`%P%AnchorOut*` properties).
    pub anchors_out: Vec<PlacementAnchor>,
    /// Position in the config file, so entries keep that order whatever their load order.
    order: usize
}

/// Role of a piece in a network drawn by the line tool.
#[derive(Clone, Debug, Reflect)]
pub struct NetworkPiece {
    /// Network name, shared by its straight and junction pieces.
    pub name: String,
    /// Junction (4 connections) or straight piece (2 connections along local X).
    pub junction: bool
}

/// Named connection point of an entry, in local grid space (x maps to world X, y to world Z).
#[derive(Clone, Debug, Reflect)]
pub struct PlacementAnchor {
    pub name: String,
    pub pos: Vec2
}

/// Entry waiting for its model to load (its footprint and anchors come from the glTF).
struct PendingEntry {
    order: usize,
    label: String,
    category: String,
    description: String,
    network: Option<NetworkPiece>,
    role: Option<String>,
    icon: Option<Handle<Texture>>,
    asset: Handle<GltfAsset>,
    asset_cut: Option<Handle<GltfAsset>>
}

/// Configuration for the placement of entities in the terrain grid.
#[derive(Resource, Default)]
pub struct PlacementConfig {
    pending_entries: Vec<PendingEntry>,
    pub labels: Vec<String>,
    pub entries: Vec<PlacementConfigEntry>,
    /// Categories in config order.
    pub categories: Vec<String>
}
impl PlacementConfig {
    /// Whether some entries still wait for their model.
    pub fn is_loading(&self) -> bool {
        !self.pending_entries.is_empty()
    }
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
    mut ui_textures: Option<ResMut<UITextures>>
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
    let mut categories: Vec<String> = Vec::new();
    for (order, entity) in config_entities.unwrap_or(&vec![]).iter().enumerate() {
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
        let asset_cut = entity["model_cut"]
            .as_str()
            .map(|path| asset_server.load(path.to_string()));
        let category = entity["category"].as_str().unwrap_or("Other").to_string();
        if !categories.contains(&category) {
            categories.push(category.clone());
        }
        let icon = entity["icon"].as_str().map(|path| {
            let icon: Handle<Texture> = asset_server.load(path.to_string());
            if let Some(ui_textures) = ui_textures.as_mut() {
                ui_textures.register_texture(icon.clone());
            }
            icon
        });
        entries.push(PendingEntry {
            order,
            label,
            category,
            description: entity["description"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            role: entity["role"].as_str().map(str::to_string),
            network: entity["network"].as_str().map(|name| NetworkPiece {
                name: name.to_string(),
                junction: entity["piece"].as_str() == Some("junction")
            }),
            icon,
            asset: asset_server.load(&model_path),
            asset_cut
        });
    }
    placement_config_struct.pending_entries = entries;
    placement_config_struct.categories = categories;
}

fn process_pending_entries(
    mut placement_config_struct: ResMut<PlacementConfig>,
    gltf_assets: Res<Assets<GltfAsset>>
) {
    if placement_config_struct.pending_entries.is_empty() {
        return;
    }
    let mut processed_entries = Vec::new();
    let mut processed_labels = Vec::new();
    let mut retry_entries = Vec::new();
    for pending in placement_config_struct.pending_entries.drain(..) {
        if let Some(gltf_asset) = gltf_assets.get(&pending.asset) {
            let properties = &gltf_asset.properties;
            let extent = match properties.get("Footprint") {
                Some((_pos, scale)) => {
                    UVec2::new((scale.x).round() as u32, (scale.z).round() as u32)
                }
                None => {
                    error!(
                        "Extent not specified for label '{}', defaulting to (1, 1).",
                        pending.label
                    );
                    UVec2::new(1, 1)
                }
            };
            let mut anchors_in = Vec::new();
            let mut anchors_out = Vec::new();
            for (anchor_name, (pos, _scale)) in properties.iter() {
                let anchor = PlacementAnchor {
                    name: anchor_name.clone(),
                    pos: Vec2::new(pos.x, pos.z)
                };
                if anchor_name.starts_with("AnchorIn") {
                    anchors_in.push(anchor);
                } else if anchor_name.starts_with("AnchorOut") {
                    anchors_out.push(anchor);
                }
            }
            processed_labels.push(pending.label.clone());
            processed_entries.push(PlacementConfigEntry {
                label: pending.label,
                category: pending.category,
                description: pending.description,
                network: pending.network,
                role: pending.role,
                icon: pending.icon,
                asset: pending.asset,
                asset_cut: pending.asset_cut,
                extent,
                anchors_in,
                anchors_out,
                order: pending.order
            });
        } else {
            retry_entries.push(pending);
        }
    }
    placement_config_struct.entries.extend(processed_entries);
    placement_config_struct.entries.sort_by_key(|e| e.order);
    placement_config_struct.labels.extend(processed_labels);
    placement_config_struct.pending_entries = retry_entries;
}
