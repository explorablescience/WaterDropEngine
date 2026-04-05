//! This module defines the `UITextures` resource, which is used to manage textures that are registered for use in the UI.
//! The `UITextures` resource allows textures to be registered in the main world, extracted to the render
//! world, and then registered in the Egui renderer, with the corresponding Egui texture IDs stored in the resource for use in the UI.
//!
//! # Example
//! ```
//! // In the main world, register a texture for use in the UI
//! let texture_handle = ui_textures.register_texture(my_texture_handle);
//!! // In the UI code, get the Egui texture ID for the registered texture
//! if let Some(egui_texture_id) = ui_textures.get_texture_index(texture_handle.asset_id()) {
//!     // Use the egui_texture_id to display the texture in the UI
//!     ui.image(egui::load::SizedTexture::new(egui_texture_id, egui::vec2(100.0, 100.0)));
//! }
//! ```

use wde_logger::prelude::*;

use bevy::prelude::*;
use std::collections::HashMap;
use wde_egui::prelude::*;
use wde_renderer::prelude::*;

pub type EguiTextureId = egui::TextureId;

pub struct UITexturesPlugin;
impl Plugin for UITexturesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UITextures>();
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .init_resource::<UITextures>()
            .add_systems(Extract, extract)
            .add_systems(Render, update_images.in_set(RenderSet::Prepare));
    }
}

#[derive(Debug, Clone)]
pub enum UITextureHandle {
    Strong(Handle<Texture>),
    Weak(AssetId<Texture>)
}
impl UITextureHandle {
    pub fn asset_id(&self) -> AssetId<Texture> {
        match self {
            UITextureHandle::Strong(handle) => handle.id(),
            UITextureHandle::Weak(asset_id) => *asset_id
        }
    }
}
impl From<UITextureHandle> for AssetId<Texture> {
    fn from(value: UITextureHandle) -> Self {
        value.asset_id()
    }
}

/// Resource that manages textures that are registered for use in the UI.
///
/// This resource allows textures to be registered in the main world, extracted to the render world, and then registered in the Egui renderer, with the corresponding Egui texture IDs stored in the resource for use in the UI.
///
/// # Example
/// ```
/// // In the main world, register a texture for use in the UI
/// let texture_handle = ui_textures.register_texture(my_texture_handle);
/// // In the UI code, get the Egui texture ID for the registered texture
/// if let Some(egui_texture_id) = ui_textures.get_texture_index(texture_handle.asset_id()) {
///     // Use the egui_texture_id to display the texture in the UI
///     ui.image(egui::load::SizedTexture::new(egui_texture_id, egui::vec2(100.0, 100.0)));
/// }
/// ```
#[derive(Resource)]
pub struct UITextures {
    // List of textures that are registered - Only set after extraction
    ui_handles: Option<Vec<(UITextureHandle, EguiTextureId)>>,
    /// Pointers from texture handles to texture indices
    ui_handle_map: Option<HashMap<AssetId<Texture>, EguiTextureId>>,
    // List of textures that need to be registered in Egui, and then sent to the render thread
    textures_to_extract: Option<Vec<UITextureHandle>>
}
impl Default for UITextures {
    fn default() -> Self {
        Self {
            ui_handles: Some(vec![]),
            textures_to_extract: Some(vec![]),
            ui_handle_map: Some(HashMap::new())
        }
    }
}
impl UITextures {
    /// Register a texture to be used in the UI.
    /// The texture will be extracted and sent to the render thread, and to the UI renderer automatically.
    ///
    /// # Returns
    /// A handle to the texture.
    pub fn register_texture(&mut self, texture: Handle<Texture>) -> UITextureHandle {
        let handle = UITextureHandle::Strong(texture.clone());
        match self.textures_to_extract {
            Some(ref mut textures) => textures.push(handle.clone()),
            None => {
                error!(
                    "UITextures resource's textures_to_extract field is None. Are you sure you called this method in the main world, and not in the render world?"
                );
            }
        }
        handle
    }

    /// Register a texture by its asset ID.
    /// The texture will be extracted and sent to the render thread, and to the UI renderer automatically.
    ///
    /// # Returns
    /// A handle to the texture.
    pub fn register_texture_weak(&mut self, texture: AssetId<Texture>) -> UITextureHandle {
        let handle = UITextureHandle::Weak(texture);
        match self.textures_to_extract {
            Some(ref mut textures) => textures.push(handle.clone()),
            None => {
                error!(
                    "UITextures resource's textures_to_extract field is None. Are you sure you called this method in the main world, and not in the render world?"
                );
            }
        }
        handle
    }

    /// Get the Egui texture index corresponding to a given texture handle, if it has been registered and extracted.
    pub fn get_texture_index(&self, asset_index: AssetId<Texture>) -> Option<EguiTextureId> {
        match &self.ui_handle_map {
            Some(map) => map.get(&asset_index).cloned(),
            None => None
        }
    }
}

fn extract(mut main_world: ResMut<MainWorld>, mut render_ui_textures: ResMut<UITextures>) {
    {
        // Extract textures from the main to the render world
        let mut main_ui_textures = main_world.resource_mut::<UITextures>();
        if let Some(textures_to_extract) = &main_ui_textures.textures_to_extract {
            render_ui_textures
                .textures_to_extract
                .as_mut()
                .unwrap()
                .extend(textures_to_extract.iter().cloned());
        }

        // Clear these textures
        main_ui_textures
            .textures_to_extract
            .as_mut()
            .unwrap()
            .clear();
    }

    {
        // Extract UI handles and handle map from the render world back to the main world
        let mut main_ui_textures = main_world.resource_mut::<UITextures>();
        main_ui_textures.ui_handles = render_ui_textures.ui_handles.clone();
        main_ui_textures.ui_handle_map = render_ui_textures.ui_handle_map.clone();
    }
}

fn update_images(
    egui_render_pass: ResMut<EguiRenderPassHolder>,
    render_instance: Res<RenderInstance>,
    mut ui_textures: ResMut<UITextures>,
    textures: Res<RenderAssets<GpuTexture>>
) {
    // Check if there are any textures to extract, and if the renderer is ready
    if ui_textures.textures_to_extract.as_ref().unwrap().is_empty()
        || egui_render_pass.renderer.is_none()
    {
        return;
    }
    let handles_to_extract = ui_textures.textures_to_extract.as_ref().unwrap();
    let mut remaining_handles_to_extract = vec![];

    // Extract textures to update from the resource
    let render_instance = render_instance.0.read().unwrap();
    let mut renderer = egui_render_pass.renderer.as_ref().unwrap().write().unwrap();
    let mut handles = vec![];
    let mut handles_hashmap = HashMap::new();
    for handle in handles_to_extract.iter() {
        let texture = match textures.get(handle.asset_id()) {
            Some(texture) => texture,
            None => {
                // Texture not found, skip it for now and try again next frame
                remaining_handles_to_extract.push(handle.clone());
                continue;
            }
        };
        let id = renderer.register_native_texture(
            &render_instance.device,
            &texture.texture.view,
            FilterMode::Linear
        );
        handles.push((handle.clone(), id));
        handles_hashmap.insert(handle.asset_id(), id);
        debug!(
            "Registered UI texture with asset ID {:?} and Egui texture ID {:?}",
            handle.asset_id(),
            id
        );
    }

    // Push the new handles
    ui_textures.ui_handles.as_mut().unwrap().extend(handles);
    ui_textures
        .ui_handle_map
        .as_mut()
        .unwrap()
        .extend(handles_hashmap);

    // Clear the list of textures to extract, except for the ones that were not found
    *ui_textures.textures_to_extract.as_mut().unwrap() = remaining_handles_to_extract;
}
