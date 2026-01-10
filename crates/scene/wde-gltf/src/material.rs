//! Material handling for glTF models.

use wde_pbr::assets::PbrMaterialAsset;
use bevy::prelude::*;

/// Representation of a glTF material's data.
#[derive(Debug, Clone)]
pub struct GltfMaterial {
    pub name: String,
    pub folder_path: String,
    pub base_color: [f32; 4],
    pub base_color_url: Option<String>,
    pub metallic: f32,
    pub roughness: f32,
    pub metallic_roughness_url: Option<String>,
}

impl GltfMaterial {
    /// Convert the GltfMaterial to a new PbrMaterialAsset.
    /// 
    /// # Arguments
    /// 
    /// * `world` - Mutable reference to the Bevy world to access resources.
    /// 
    /// # Returns
    /// * `Handle<PbrMaterialAsset>` - Handle to the created PbrMaterialAsset.
    pub fn to_pbr(&self, world: &mut World) -> Handle<PbrMaterialAsset> {
        // Load base color texture if available
        let aldebo_texture_handle = self.base_color_url
            .as_ref()
            .map(|texture_url| world.resource::<AssetServer>().load(format!("{}/{}", self.folder_path, texture_url)));

        // Load metallic-roughness texture if available
        let metallic_roughness_texture_handle = self.metallic_roughness_url
            .as_ref()
            .map(|texture_url| world.resource::<AssetServer>().load(format!("{}/{}", self.folder_path, texture_url)));

        // Create and add the material to the asset server
        world
            .resource_mut::<Assets<PbrMaterialAsset>>()
            .add(PbrMaterialAsset {
                label: "gltf_material".to_string(),
                albedo: (self.base_color[0], self.base_color[1], self.base_color[2], self.base_color[3]),
                specular: self.roughness,
                albedo_t: aldebo_texture_handle,
                specular_t: metallic_roughness_texture_handle
            })
    }
}

/// Default implementation for MaterialData, providing a white material.
impl Default for GltfMaterial {
    fn default() -> Self {
        GltfMaterial {
            name: "default_material".to_string(),
            folder_path: "".to_string(),
            base_color: [1.0, 1.0, 1.0, 1.0],
            base_color_url: None,
            metallic: 0.0,
            roughness: 1.0,
            metallic_roughness_url: None,
        }
    }
}
