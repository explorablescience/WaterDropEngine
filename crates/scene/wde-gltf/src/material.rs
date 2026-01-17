//! Material handling for glTF models.

use wde_pbr::assets::PbrMaterialAsset;
use bevy::prelude::*;
use wde_renderer::assets::TextureLoaderSettings;

/// Representation of a glTF material's data.
#[derive(Debug, Clone)]
pub struct GltfMaterial {
    pub name: String,
    pub folder_path: String,
    pub base_color: [f32; 4],
    pub base_color_tex_url: Option<String>,
    pub metallic: f32,
    pub roughness: f32,
    pub metallic_roughness_tex_url: Option<String>,
    pub normal_tex_url: Option<String>,
    pub occlusion_tex_url: Option<String>,
}

impl GltfMaterial {
    /// Convert the GltfMaterial to a new PbrMaterialAsset.
    /// 
    /// # Arguments
    /// 
    /// * `asset_server` - Reference to the Bevy asset server for loading textures.
    /// 
    /// # Returns
    /// * `Handle<PbrMaterialAsset>` - Handle to the created PbrMaterialAsset.
    pub fn to_pbr(&self, asset_server: &AssetServer) -> Handle<PbrMaterialAsset> {
        // Load base color texture if available
        let aldebo_texture_handle = self.base_color_tex_url
            .as_ref()
            .map(|texture_url| {
                let label = format!("gltf_albedo_{}", self.name);
                asset_server.load_with_settings(format!("{}/{}", self.folder_path, texture_url), 
                    move |settings: &mut TextureLoaderSettings| {
                        settings.label = label.clone();
                    }
                )
            });

        // Load metallic-roughness texture if available
        let metallic_roughness_texture_handle = self.metallic_roughness_tex_url
            .as_ref()
            .map(|texture_url| {
                let label = format!("gltf_metallic_roughness_{}", self.name);
                asset_server.load_with_settings(format!("{}/{}", self.folder_path, texture_url), 
                    move |settings: &mut TextureLoaderSettings| {
                        settings.label = label.clone();
                    }
                )
            });

        // Load normal texture if available
        let normal_texture_handle = self.normal_tex_url
            .as_ref()
            .map(|texture_url| {
                let label = format!("gltf_normal_{}", self.name);
                asset_server.load_with_settings(format!("{}/{}", self.folder_path, texture_url), 
                    move |settings: &mut TextureLoaderSettings| {
                        settings.label = label.clone();
                    }
                )
            });

        // Load occlusion texture if available
        let occlusion_texture_handle = self.occlusion_tex_url
            .as_ref()
            .map(|texture_url| {
                let label = format!("gltf_occlusion_{}", self.name);
                asset_server.load_with_settings(format!("{}/{}", self.folder_path, texture_url), 
                    move |settings: &mut TextureLoaderSettings| {
                        settings.label = label.clone();
                    }
                )
            });

        // Create and add the material to the asset server
        asset_server
            .add(PbrMaterialAsset {
                label: format!("gltf_material_{}", self.name),

                albedo: (self.base_color[0], self.base_color[1], self.base_color[2], self.base_color[3]),
                albedo_t: aldebo_texture_handle,

                metallic: self.metallic,
                roughness: self.roughness,
                metallic_roughness_t: metallic_roughness_texture_handle,

                normal_t: normal_texture_handle,
                occlusion_t: occlusion_texture_handle,
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
            base_color_tex_url: None,

            metallic: 0.0,
            roughness: 1.0,
            metallic_roughness_tex_url: None,
            
            normal_tex_url: None,
            occlusion_tex_url: None,
        }
    }
}
