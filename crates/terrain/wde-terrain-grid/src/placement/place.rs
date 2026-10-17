use std::collections::BTreeSet;

use wde_terrain::TERRAIN_BUILDINGS_COLLIDER_GROUP;

use crate::{
    core::{entries::PlacementConfigEntry, grid::GridEntityEvent},
    placement::{
        TerrainCursorPos,
        connections::connected_anchors,
        cutaway::BuildingVariant,
        modules::{InteriorModule, module_host, spawn_module}
    }
};
use bevy::prelude::*;
use wde_editor::prelude::*;
use wde_gltf::prelude::*;
use wde_pbr::prelude::*;
use wde_physics::prelude::*;
use wde_renderer::prelude::{Color, *};

use crate::{
    core::entries::PlacementConfig,
    placement::{TerrainPlacementManager, TerrainPlacementMode},
    prelude::{Grid, GridEntity, GridRotation},
    render::ghost::ghost_material::GhostMaterial
};

#[derive(Component)]
pub(crate) struct GhostMaterialStorer {
    ok_material: Handle<GhostMaterial>,
    nok_material: Handle<GhostMaterial>,
    current_validity: bool
}

const CARD_SIZE: ui::egui::Vec2 = ui::egui::vec2(96.0, 122.0);
const CARD_ICON_SIZE: f32 = 84.0;

pub fn ui_place_entity(
    mut commands: Commands,
    ui: &mut ui::egui::Ui,
    manager: &mut TerrainPlacementManager,
    config: &PlacementConfig,
    gltf_models: &Assets<GltfAsset>,
    asset_server: &AssetServer,
    ui_textures: Option<&UITextures>
) {
    use ui::egui;

    if config.entries.is_empty() {
        ui.label(egui::RichText::new("Loading buildings...").weak());
        return;
    }

    // Category tabs
    let category = manager
        .place_selected_category
        .get_or_insert_with(|| config.categories.first().cloned().unwrap_or_default())
        .clone();
    ui.horizontal_wrapped(|ui| {
        for cat in config.categories.iter() {
            // One card per building, and one per network (its pieces are drawn, not placed)
            let count = config
                .entries
                .iter()
                .filter(|e| &e.category == cat && e.network.as_ref().is_none_or(|n| !n.junction))
                .count();
            let tab = egui::Button::new(format!("{cat}  {count}"))
                .selected(*cat == category)
                .min_size(egui::vec2(0.0, 22.0));
            if ui.add(tab).clicked() {
                manager.place_selected_category = Some(cat.clone());
            }
        }
    });
    ui.add_space(4.0);

    // Networks of this category, drawn with the line tool instead of placed piece by piece
    let mut networks: Vec<&PlacementConfigEntry> = Vec::new();
    for entry in config.entries.iter().filter(|e| e.category == category) {
        if let Some(network) = &entry.network
            && !network.junction
            && !networks
                .iter()
                .any(|n| n.network.as_ref().map(|n| &n.name) == Some(&network.name))
        {
            networks.push(entry);
        }
    }

    // Building cards
    let old_label = manager.place_selected_entry_label.clone();
    egui::ScrollArea::vertical()
        .max_height(CARD_SIZE.y * 2.0 + 24.0)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                let icon_of = |entry: &PlacementConfigEntry| {
                    entry
                        .icon
                        .as_ref()
                        .zip(ui_textures)
                        .and_then(|(icon, textures)| textures.get_texture_index(icon.id()))
                };
                for straight in &networks {
                    let network = straight.network.as_ref().unwrap().name.clone();
                    let active = manager.line_network.as_ref() == Some(&network);
                    let label = format!("{}s", capitalize(&network));
                    let response = building_card(ui, &label, "draw", icon_of(straight), active)
                        .on_hover_ui(|ui| {
                            ui.set_max_width(240.0);
                            ui.strong(&label);
                            ui.label(format!(
                                "Drag on the terrain to lay {network}s. Straights, corners and \
                                 junctions are chosen automatically, and existing {network}s \
                                 are joined."
                            ));
                        });
                    if response.clicked() {
                        manager.line_network = (!active).then_some(network);
                        manager.place_selected_entry_label = None;
                    }
                }
                for entry in config
                    .entries
                    .iter()
                    .filter(|e| e.category == category && e.network.is_none())
                {
                    let selected = old_label.as_deref() == Some(entry.label.as_str());
                    let badge = format!("{}×{}", entry.extent.x, entry.extent.y);
                    let response =
                        building_card(ui, &entry.label, &badge, icon_of(entry), selected)
                            .on_hover_ui(|ui| building_tooltip(ui, entry));
                    if response.clicked() {
                        manager.place_selected_entry_label =
                            (!selected).then(|| entry.label.clone());
                        manager.line_network = None;
                    }
                }
            });
        });
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        manager.place_selected_entry_label = None;
        manager.line_network = None;
    }

    // Selection summary
    ui.separator();
    match config
        .entries
        .iter()
        .find(|e| Some(&e.label) == manager.place_selected_entry_label.as_ref())
    {
        Some(entry) => {
            ui.horizontal(|ui| {
                ui.strong(&entry.label);
                ui.label(
                    egui::RichText::new(format!("{}×{} tiles", entry.extent.x, entry.extent.y))
                        .weak()
                );
            });
            ui.label(
                egui::RichText::new(
                    "Left click: place   •   Middle click: rotate   •   Esc: cancel"
                )
                .small()
                .weak()
            );
        }
        None => match &manager.line_network {
            Some(network) => {
                ui.strong(format!("Drawing {network}s"));
                ui.label(
                    egui::RichText::new(
                        "Drag: draw a line   •   Right click: cancel the line   •   Esc: stop"
                    )
                    .small()
                    .weak()
                );
            }
            None => {
                ui.label(egui::RichText::new("Select a building to place.").weak());
            }
        }
    }

    if manager.place_selected_entry_label.is_none() {
        reset_placement_tool(&mut commands, manager);
        return;
    }

    // Update placement entry based on selected label
    if old_label != manager.place_selected_entry_label {
        let entry = config
            .entries
            .iter()
            .find(|e| &e.label == manager.place_selected_entry_label.as_ref().unwrap())
            .unwrap();
        commands.entity(manager.entity).despawn_children();
        spawn_new_ghost_entity(&mut commands, entry, gltf_models, asset_server, manager);
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    chars
        .next()
        .map(|c| c.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

/// Clickable card with an icon, a name and a small badge (e.g. the footprint).
fn building_card(
    ui: &mut ui::egui::Ui,
    label: &str,
    badge: &str,
    icon: Option<ui::egui::TextureId>,
    selected: bool
) -> ui::egui::Response {
    use ui::egui;

    let (rect, response) = ui.allocate_exact_size(CARD_SIZE, egui::Sense::click());
    if !ui.is_rect_visible(rect) {
        return response;
    }
    let visuals = ui.style().interact_selectable(&response, selected);
    let painter = ui.painter();
    painter.rect(
        rect,
        6.0,
        visuals.weak_bg_fill,
        visuals.bg_stroke,
        egui::StrokeKind::Inside
    );

    let icon_rect = egui::Rect::from_center_size(
        egui::pos2(rect.center().x, rect.top() + 4.0 + CARD_ICON_SIZE / 2.0),
        egui::Vec2::splat(CARD_ICON_SIZE)
    );
    match icon {
        Some(texture) => {
            painter.image(
                texture,
                icon_rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE
            );
        }
        None => {
            let initial = label.chars().next().unwrap_or('?').to_string();
            painter.text(
                icon_rect.center(),
                egui::Align2::CENTER_CENTER,
                initial,
                egui::FontId::proportional(32.0),
                visuals.text_color()
            );
        }
    }

    // Footprint badge
    let badge = painter.layout_no_wrap(
        badge.to_string(),
        egui::FontId::monospace(10.0),
        ui.visuals().weak_text_color()
    );
    let badge_rect = egui::Rect::from_min_size(
        egui::pos2(rect.right() - badge.size().x - 10.0, rect.top() + 4.0),
        badge.size() + egui::vec2(6.0, 2.0)
    );
    painter.rect_filled(badge_rect, 3.0, ui.visuals().extreme_bg_color);
    painter.galley(
        badge_rect.min + egui::vec2(3.0, 1.0),
        badge,
        egui::Color32::PLACEHOLDER
    );

    // Name, centered and wrapped on two lines at most
    let mut job = egui::text::LayoutJob::simple(
        label.to_string(),
        egui::FontId::proportional(12.0),
        visuals.text_color(),
        rect.width() - 8.0
    );
    job.halign = egui::Align::Center;
    job.wrap.max_rows = 2;
    let name = painter.layout_job(job);
    painter.galley(
        egui::pos2(rect.center().x, icon_rect.bottom() + 2.0),
        name,
        visuals.text_color()
    );
    response
}

fn building_tooltip(ui: &mut ui::egui::Ui, entry: &PlacementConfigEntry) {
    use ui::egui;

    ui.set_max_width(240.0);
    ui.strong(&entry.label);
    if !entry.description.is_empty() {
        ui.label(&entry.description);
    }
    ui.add_space(2.0);
    ui.label(
        egui::RichText::new(format!(
            "Footprint: {}×{} tiles",
            entry.extent.x, entry.extent.y
        ))
        .small()
        .weak()
    );
    if !entry.anchors_in.is_empty() {
        ui.label(
            egui::RichText::new(format!("Tunnel connections: {}", entry.anchors_in.len()))
                .small()
                .weak()
        );
    }
    if !entry.anchors_out.is_empty() {
        ui.label(
            egui::RichText::new(format!("Path connections: {}", entry.anchors_out.len()))
                .small()
                .weak()
        );
    }
    if entry.asset_cut.is_some() {
        ui.label(egui::RichText::new("Has a cutaway view").small().weak());
    }
}

pub fn handle_new_entity_to_place(
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    mut manager: ResMut<TerrainPlacementManager>,
    gltf_models: Res<Assets<GltfAsset>>
) {
    let Some(ref entry) = manager.new_entry_to_place else {
        return;
    };
    let label = entry.label.clone();
    spawn_new_ghost_entity(
        &mut commands,
        &entry.clone(),
        &gltf_models,
        &asset_server,
        &mut manager
    );
    manager.place_selected_entry_label = Some(label);
    manager.new_entry_to_place = None;
}

fn spawn_new_ghost_entity(
    commands: &mut Commands,
    entry: &PlacementConfigEntry,
    gltf_models: &Assets<GltfAsset>,
    asset_server: &AssetServer,
    manager: &mut TerrainPlacementManager
) {
    manager.place_selected_entry = Some(entry.clone());
    manager.ghost_connections = None;

    // Not loaded yet: `place_update` spawns the parts once it is
    if let Some(model) = gltf_models.get(&entry.asset) {
        let connected = BTreeSet::new();
        spawn_ghost_parts(
            commands,
            entry,
            model,
            asset_server,
            manager.entity,
            &connected,
            true
        );
        manager.ghost_connections = Some(connected);
    }
}

/// Spawns the ghost meshes of `entry` for the given connected anchors.
pub(crate) fn spawn_ghost_parts(
    commands: &mut Commands,
    entry: &PlacementConfigEntry,
    model: &GltfAsset,
    asset_server: &AssetServer,
    parent: Entity,
    connected: &BTreeSet<String>,
    valid: bool
) {
    let children = model
        .models_where(|anchor| connected.contains(anchor))
        .map(|(mesh, material)| {
            // Create material
            let ok_material = asset_server.add(GhostMaterial {
                overlay: Color::LinearRgba(0.35, 0.85, 1.0, 0.35),
                desaturate: 0.7,
                rim_power: 2.5,
                rim_strength: 1.0,
                pbr_material: Some(material.clone()),
                ..Default::default()
            });
            let nok_material = asset_server.add(GhostMaterial {
                overlay: Color::LinearRgba(1.0, 0.18, 0.12, 0.42),
                desaturate: 0.85,
                rim_power: 2.5,
                rim_strength: 1.2,
                pbr_material: Some(material.clone()),
                ..Default::default()
            });
            let initial = match valid {
                true => ok_material.clone(),
                false => nok_material.clone()
            };

            // Spawn entity
            commands
                .spawn((
                    Name::new(format!("Placement Entity - {}", entry.label)),
                    Transform::default(),
                    Mesh3d(mesh.clone()),
                    PbrMaterial3d(initial),
                    GhostMaterialStorer {
                        ok_material,
                        nok_material,
                        current_validity: valid
                    }
                ))
                .id()
        })
        .collect::<Vec<Entity>>();
    commands.entity(parent).add_children(&children);
}

#[allow(clippy::too_many_arguments)]
pub fn place_update(
    terrain_cursor_pos: Res<TerrainCursorPos>,
    mut commands: Commands,
    mut manager: ResMut<TerrainPlacementManager>,
    mut local_rot: Local<GridRotation>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    grid: Res<Grid>,
    children_query: Query<&Children>,
    mut material_query: Query<(&Mesh3d, &mut GhostMaterialStorer)>,
    gltf_models: Res<Assets<GltfAsset>>,
    mut grid_entity_events: MessageWriter<GridEntityEvent>,
    asset_server: Res<AssetServer>,
    placed: Query<&GridEntity, With<BuildingVariant>>,
    modules: Query<&GridEntity, With<InteriorModule>>
) {
    if manager.place_selected_entry.is_none()
        || (manager.mode != TerrainPlacementMode::Place
            && manager.mode != TerrainPlacementMode::Move)
    {
        return;
    }

    // Toggle rotation on right click
    if mouse_input.just_pressed(MouseButton::Middle) {
        *local_rot = match *local_rot {
            GridRotation::R0 => GridRotation::R90,
            GridRotation::R90 => GridRotation::R180,
            GridRotation::R180 => GridRotation::R270,
            GridRotation::R270 => GridRotation::R0
        };
    }

    // Cast the ray in the physics world
    if let Some(pos) = terrain_cursor_pos.pos() {
        // Move the placement entity to the hit point with the correct rotation (modules: on the
        // floor of the building around them)
        let grid_entity = GridEntity::new(
            Vec2::new(pos.x, pos.z),
            *local_rot,
            manager.place_selected_entry.clone().unwrap()
        );
        let is_module = grid_entity.entry().module;
        let host = match is_module {
            true => module_host(&grid, &placed, modules.iter(), &grid_entity),
            false => None
        };
        commands.entity(manager.entity).insert(
            Transform::from_rotation(Quat::from_rotation_y(local_rot.rotation())).with_translation(
                Vec3::new(
                    grid_entity.center().x,
                    host.map_or(0.0, |(_, floor)| floor),
                    grid_entity.center().y
                )
            )
        );

        // Preview the variant matching the neighbors at this position
        let is_valid = match is_module {
            true => host.is_some(),
            false => grid.is_area_free(&grid_entity)
        };
        let connected = match is_module {
            true => BTreeSet::new(),
            false => connected_anchors(&grid_entity, placed.iter())
        };
        let mut respawned = false;
        if manager.ghost_connections.as_ref() != Some(&connected) {
            let entry = manager.place_selected_entry.clone().unwrap();
            if let Some(model) = gltf_models.get(&entry.asset) {
                commands.entity(manager.entity).despawn_children();
                spawn_ghost_parts(
                    &mut commands,
                    &entry,
                    model,
                    &asset_server,
                    manager.entity,
                    &connected,
                    is_valid
                );
                manager.ghost_connections = Some(connected);
                respawned = true;
            }
        }

        // Check for placement validity and update ghost material accordingly. Skipped when the
        // parts were just respawned: `Children` still lists the old, despawned ones this frame.
        let ghost_parts = match respawned {
            true => Vec::new(),
            false => children_query
                .get(manager.entity)
                .map(|c| c.iter().collect::<Vec<_>>())
                .unwrap_or_default()
        };
        for child in ghost_parts {
            if let Ok((_, mut material_storer)) = material_query.get_mut(child) {
                if material_storer.current_validity == is_valid {
                    continue; // No change in validity, skip
                }
                let new_material = if is_valid {
                    material_storer.ok_material.clone()
                } else {
                    material_storer.nok_material.clone()
                };
                commands
                    .entity(child)
                    .try_insert(PbrMaterial3d(new_material));
                material_storer.current_validity = is_valid;
            }
        }

        // Place the entity on left click if the position is valid
        if mouse_input.just_pressed(MouseButton::Left) && is_valid {
            let grid_entity = GridEntity::new(
                Vec2::new(pos.x, pos.z),
                *local_rot,
                manager.place_selected_entry.clone().unwrap()
            );

            match host {
                Some((host, floor)) => {
                    spawn_module(
                        &mut commands,
                        &grid,
                        &gltf_models,
                        &grid_entity,
                        host,
                        floor
                    );
                }
                None => {
                    spawn_building(
                        &mut commands,
                        &grid,
                        &gltf_models,
                        &grid_entity,
                        &mut grid_entity_events
                    );
                }
            }

            // Reset placement tool
            reset_placement_tool(&mut commands, &mut manager);
        }
    }
}

/// Spawns a placed building (parent with its collider; meshes follow from its variant) and
/// notifies the grid. Returns `None` if its model is not loaded yet.
pub(crate) fn spawn_building(
    commands: &mut Commands,
    grid: &Grid,
    gltf_models: &Assets<GltfAsset>,
    grid_entity: &GridEntity,
    grid_entity_events: &mut MessageWriter<GridEntityEvent>
) -> Option<Entity> {
    let entry = grid_entity.entry();
    let model = gltf_models.get(&entry.asset)?;
    let bbox = &model.bbox;
    let parent = commands
        .spawn((
            Name::new(entry.label.clone()),
            Transform::from_rotation(Quat::from_rotation_y(grid_entity.rotation().rotation()))
                .with_translation(Vec3::new(
                    grid_entity.center().x,
                    0.0,
                    grid_entity.center().y
                )),
            grid_entity.clone(),
            BuildingVariant::default(),
            ChildOf(grid.get_parent()?)
        ))
        .id();

    // Collider from the model bounding box
    let bbox_extent = bbox.max - bbox.min;
    commands.spawn((
        Name::new("Collider"),
        Transform::from_translation(bbox.min + bbox_extent / 2.0),
        Collider::from(BoxCollider::new(bbox_extent).with_group(TERRAIN_BUILDINGS_COLLIDER_GROUP)),
        ChildOf(parent)
    ));
    model.spawn_lights(commands, parent);

    grid_entity_events.write(GridEntityEvent::Placed {
        entity: parent,
        grid_entity: Box::new(grid_entity.clone())
    });
    Some(parent)
}

/// Despawns a placed building and frees its grid tiles.
pub(crate) fn remove_building(
    commands: &mut Commands,
    entity: Entity,
    grid_entity_events: &mut MessageWriter<GridEntityEvent>
) {
    commands.entity(entity).despawn();
    grid_entity_events.write(GridEntityEvent::Removed { entity });
}

fn reset_placement_tool(commands: &mut Commands, manager: &mut TerrainPlacementManager) {
    manager.place_selected_entry_label = None;
    manager.place_selected_entry = None;
    manager.ghost_connections = None;
    commands
        .entity(manager.entity)
        .insert(Transform::from_translation(Vec3::MIN));
    commands.entity(manager.entity).despawn_children();
}
