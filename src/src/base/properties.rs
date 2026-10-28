use std::collections::HashMap;

use wde::prelude::*;

use bevy::prelude::*;

use crate::base::electricity::{ElectricityConsumer, ElectricityProducer};

const BUILDINGS_CONFIG_VERSION: u32 = 1;
const BUILDINGS_CONFIG_PATH: &str = "core/config/buildings.json";

/// Gameplay properties of a building or module, read from the buildings configuration.
#[derive(Clone, Default, Debug)]
pub struct BuildingProperties {
    pub electricity_production: f32,
    pub electricity_consumption: f32
}

/// Properties of each building, by placement label.
#[derive(Resource, Default)]
pub struct BuildingsConfig(pub HashMap<String, BuildingProperties>);

pub(super) fn read_buildings_file(mut config: ResMut<BuildingsConfig>) {
    let file = match serialize::parse_file(BUILDINGS_CONFIG_PATH) {
        Ok(file) => file.data,
        Err(err) => {
            error!(
                "Failed to load buildings configuration file at '{}': {}",
                BUILDINGS_CONFIG_PATH, err
            );
            return;
        }
    };
    if file["version"].as_u64().unwrap_or(0) != BUILDINGS_CONFIG_VERSION as u64 {
        error!(
            "Unsupported buildings configuration version: {}. Expected version: {}",
            file["version"], BUILDINGS_CONFIG_VERSION
        );
        return;
    }

    for building in file["buildings"].as_array().unwrap_or(&vec![]) {
        let Some(label) = building["label"].as_str() else {
            error!("Building without label in buildings config: {:?}", building);
            continue;
        };
        let amount = |kind: &str, resource: &str| building[kind][resource].as_f64().unwrap_or(0.0);
        config.0.insert(
            label.to_string(),
            BuildingProperties {
                electricity_production: amount("produces", "electricity") as f32,
                electricity_consumption: amount("consumes", "electricity") as f32
            }
        );
    }
}

/// Gives newly placed buildings and modules the components matching their properties.
pub(super) fn insert_properties(
    mut commands: Commands,
    config: Res<BuildingsConfig>,
    added: Query<(Entity, &GridEntity), Added<GridEntity>>
) {
    for (entity, building) in &added {
        let Some(properties) = config.0.get(&building.entry().label) else {
            continue;
        };
        let mut entity = commands.entity(entity);
        if properties.electricity_production > 0.0 {
            entity.insert(ElectricityProducer(properties.electricity_production));
        }
        if properties.electricity_consumption > 0.0 {
            entity.insert(ElectricityConsumer(properties.electricity_consumption));
        }
    }
}
