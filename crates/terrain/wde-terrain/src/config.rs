use wde_logger::prelude::*;

use bevy::prelude::*;
use wde_scene::prelude::serialize;

use crate::{
    manager::{CHUNK_COUNT, CHUNK_RENDER_SUBDIVISIONS, CHUNK_SIZE},
    render::dependencies::terrain_buffer::TERRAIN_MATERIAL_COUNT
};

const TERRAIN_CONFIG_VERSION: u64 = 1;
/// Description of a terrain, in its folder.
pub const TERRAIN_CONFIG_FILE: &str = "config/terrain.json";

/// Description of a terrain, read from `res/<path>/config/terrain.json`: how its heightmaps encode
/// heights, where its maps are, its ground materials, and the rules deriving its slope and
/// buildable masks (see
/// [`Terrain::slope_at`](crate::prelude::Terrain::slope_at)). The chunk layout it declares must
/// match the one the engine is built for ([`CHUNK_COUNT`], [`CHUNK_SIZE`],
/// [`CHUNK_RENDER_SUBDIVISIONS`]).
#[derive(Clone, Debug, Reflect)]
pub struct TerrainConfig {
    pub label: String,
    /// World height of a heightmap value of 0.
    pub min_height: f32,
    /// Height range covered by the heightmaps, from `min_height`.
    pub height_range: f32,
    /// Heightmap of chunk `(x, z)`, relative to the terrain folder, with `{x}` and `{z}`
    /// placeholders.
    pub heightmap: String,
    /// Splat map `k` of chunk `(x, z)`, with `{x}`, `{z}` and `{k}` placeholders.
    pub splatmap: String,
    /// Ground materials in splat map order (map `k` RGBA blends materials `4k..4k+3`), at most
    /// [`TERRAIN_MATERIAL_COUNT`].
    pub materials: Vec<TerrainMaterial>,
    pub masks: TerrainMaskRules
}

/// Ground material blended by the splat maps.
#[derive(Clone, Debug, Reflect)]
pub struct TerrainMaterial {
    /// Folder in `res/` of its textures: `albedo`, `normal`, `roughness`, `ambient_occlusion` and
    /// `displacement` PNGs, all of the same size.
    pub path: String,
    /// World size covered by one repeat of its textures.
    pub tiling: f32,
    /// How much its relief pushes it over the others where they blend (see
    /// [`TerrainRenderSettings::height_blend`](crate::prelude::TerrainRenderSettings::height_blend)).
    pub height_blend: f32
}

/// Rules deriving the masks from the heights.
#[derive(Clone, Copy, Debug, Reflect)]
pub struct TerrainMaskRules {
    /// Height difference from 0 tolerated on buildable ground: buildings stand at height 0.
    pub build_height_tolerance: f32,
    /// Steepest buildable ground (height change per unit of distance).
    pub build_max_slope: f32,
    /// Distance kept between buildable ground and any ground that is not.
    pub build_margin: f32,
    /// Steepest walkable ground.
    pub walk_max_slope: f32
}

impl Default for TerrainConfig {
    fn default() -> Self {
        Self {
            label: String::new(),
            min_height: -32.0,
            height_range: 72.0,
            heightmap: "heightmaps/heightmap_{x}_{z}.png".to_string(),
            splatmap: "splatmaps/splatmap_{x}_{z}-{k}.png".to_string(),
            materials: Vec::new(),
            masks: TerrainMaskRules {
                build_height_tolerance: 0.05,
                build_max_slope: 0.05,
                build_margin: 1.0,
                walk_max_slope: 0.9
            }
        }
    }
}

impl TerrainConfig {
    /// Reads the config of the terrain in `res/<path>/`, falling back to the defaults on error.
    pub fn load(path: &str) -> Self {
        let file = format!("{path}/{TERRAIN_CONFIG_FILE}");
        match Self::parse(&file) {
            Ok(config) => config,
            Err(e) => {
                error!("Failed to load terrain config '{file}': {e}. Using the defaults.");
                Self::default()
            }
        }
    }

    fn parse(file: &str) -> Result<Self, String> {
        let parsed = serialize::parse_file(file).map_err(|e| e.to_string())?;
        let data = parsed.data;
        if data["version"].as_u64() != Some(TERRAIN_CONFIG_VERSION) {
            return Err(format!(
                "unsupported version {}, expected {TERRAIN_CONFIG_VERSION}",
                data["version"]
            ));
        }
        let number = |section: &str, key: &str| -> Result<f32, String> {
            data[section][key]
                .as_f64()
                .map(|v| v as f32)
                .ok_or_else(|| format!("missing number '{section}.{key}'"))
        };
        let text = |section: &str, key: &str| -> Result<String, String> {
            data[section][key]
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| format!("missing string '{section}.{key}'"))
        };

        let layout = (
            number("layout", "chunk_count")? as u32,
            number("layout", "chunk_size")?,
            number("layout", "chunk_resolution")? as u32
        );
        if layout != (CHUNK_COUNT, CHUNK_SIZE, CHUNK_RENDER_SUBDIVISIONS) {
            return Err(format!(
                "chunk layout (count, size, resolution) {layout:?} differs from the engine's ({CHUNK_COUNT}, {CHUNK_SIZE}, {CHUNK_RENDER_SUBDIVISIONS})"
            ));
        }

        let materials = data["materials"].as_array().cloned().unwrap_or_default();
        if materials.len() > TERRAIN_MATERIAL_COUNT {
            return Err(format!(
                "{} materials, at most {TERRAIN_MATERIAL_COUNT} are blended",
                materials.len()
            ));
        }
        let materials = materials
            .iter()
            .map(|material| {
                Ok(TerrainMaterial {
                    path: material["path"]
                        .as_str()
                        .ok_or("material without 'path'")?
                        .to_string(),
                    tiling: material["tiling"].as_f64().unwrap_or(10.0) as f32,
                    height_blend: material["height_blend"].as_f64().unwrap_or(0.5) as f32
                })
            })
            .collect::<Result<_, String>>()?;

        Ok(Self {
            label: parsed.label,
            min_height: number("heights", "min")?,
            height_range: number("heights", "range")?,
            heightmap: text("maps", "heightmap")?,
            splatmap: text("maps", "splatmap")?,
            materials,
            masks: TerrainMaskRules {
                build_height_tolerance: number("masks", "build_height_tolerance")?,
                build_max_slope: number("masks", "build_max_slope")?,
                build_margin: number("masks", "build_margin")?,
                walk_max_slope: number("masks", "walk_max_slope")?
            }
        })
    }

    /// Path of the heightmap of a chunk, relative to the terrain folder.
    pub fn heightmap_file(&self, pos: IVec2) -> String {
        self.heightmap
            .replace("{x}", &pos.x.to_string())
            .replace("{z}", &pos.y.to_string())
    }

    /// Path of splat map `k` of a chunk, relative to the terrain folder.
    pub fn splatmap_file(&self, pos: IVec2, k: u32) -> String {
        self.splatmap
            .replace("{x}", &pos.x.to_string())
            .replace("{z}", &pos.y.to_string())
            .replace("{k}", &k.to_string())
    }
}
