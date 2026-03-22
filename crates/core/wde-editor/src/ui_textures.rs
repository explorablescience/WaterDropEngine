
use std::collections::HashMap;

use wde_egui::prelude::*;
use wde_logger::prelude::*;
use bevy::prelude::*;
use wde_renderer::{core::MainWorld, prelude::*};

pub type EguiTextureId = egui::TextureId;

pub struct UITexturesPlugin;
impl Plugin for UITexturesPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(UITextures { ui_handles: Some(vec![]), textures_to_extract: Some(vec![]), ui_handle_map: Some(HashMap::new()) });
        app.get_sub_app_mut(RenderApp).unwrap()
            .insert_resource(UITextures { ui_handles: Some(vec![]), textures_to_extract: None, ui_handle_map: Some(HashMap::new()) })
            .add_systems(Extract, extract)
            .add_systems(Render, update_images.in_set(RenderSet::Prepare));
    }
}


#[derive(Debug, Clone)]
pub enum UITextureHandle {
    Strong(Handle<Texture>),
    Weak(AssetId<Texture>),
}
impl UITextureHandle {
    pub fn asset_id(&self) -> AssetId<Texture> {
        match self {
            UITextureHandle::Strong(handle) => handle.id(),
            UITextureHandle::Weak(asset_id) => *asset_id,
        }
    }
}
impl From<UITextureHandle> for AssetId<Texture> {
    fn from(value: UITextureHandle) -> Self {
        value.asset_id()
    }
}


#[derive(Resource, Default)]
pub struct UITextures {
    // List of textures that are registered - Only set after extraction
    ui_handles: Option<Vec<(UITextureHandle, EguiTextureId)>>,
    /// Pointers from texture handles to texture indices
    ui_handle_map: Option<HashMap<AssetId<Texture>, EguiTextureId>>,
    // List of textures that need to be registered in Egui, and then sent to the render thread
    textures_to_extract: Option<Vec<UITextureHandle>>
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
                error!("UITextures resource's textures_to_extract field is None. Are you sure you called this method in the main world, and not in the render world?");
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
                error!("UITextures resource's textures_to_extract field is None. Are you sure you called this method in the main world, and not in the render world?");
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

fn extract(
    mut main_world: ResMut<MainWorld>,
    mut render_ui_textures: ResMut<UITextures>,
) {
    {
        // Extract textures from the main to the render world
        let main_ui_textures = main_world.resource::<UITextures>();
        render_ui_textures.textures_to_extract = main_ui_textures.textures_to_extract.clone();
    }

    {
        // Clear the list of textures to extract in the main world, to avoid re-extracting them every frame
        let mut main_ui_textures = main_world.resource_mut::<UITextures>();
        main_ui_textures.textures_to_extract.as_mut().unwrap().clear();
    }

    {
        // Extract UI handles and handle map from the render world back to the main world
        let mut main_ui_textures = main_world.resource_mut::<UITextures>();
        main_ui_textures.ui_handles = render_ui_textures.ui_handles.clone();
        main_ui_textures.ui_handle_map = render_ui_textures.ui_handle_map.clone();
    }
}

fn update_images(
    egui_render_pass: ResMut<EguiRenderPass>,
    render_instance: Res<RenderInstance>,
    mut ui_textures: ResMut<UITextures>,
    textures: Res<RenderAssets<GpuTexture>>
) {
    let render_instance = render_instance.0.read().unwrap();
    
    // Extract textures to update from the resource
    let mut renderer = egui_render_pass.renderer.as_ref().unwrap().write().unwrap();
    let mut handles = vec![];
    let mut handles_hashmap = HashMap::new();
    for handle in ui_textures.textures_to_extract.as_ref().unwrap() {
        let texture = match textures.get(handle.asset_id()) {
            Some(texture) => texture,
            None => {
                error!("Texture with asset ID {:?} not found in RenderAssets<GpuTexture>", handle.asset_id());
                continue;
            }
        };
        let id = renderer.register_native_texture(&render_instance.device, &texture.texture.view, FilterMode::Linear);
        handles.push((handle.clone(), id));
        handles_hashmap.insert(handle.asset_id(), id);
        debug!("Registered UI texture with asset ID {:?} and Egui texture ID {:?}", handle.asset_id(), id);
    }

    // Remove the handles that already exist in the UI textures resource, to force override
    if let Some(existing_handles) = &ui_textures.ui_handles {
        for (handle, id) in existing_handles {
            if handles_hashmap.contains_key(&handle.asset_id()) {
                debug!("Overriding existing UI texture with asset ID {:?} and Egui texture ID {:?}", handle.asset_id(), id);
                renderer.free_texture(id);
            }
        }
    }

    // Push the new handles
    ui_textures.ui_handles.as_mut().unwrap().extend(handles);
    ui_textures.ui_handle_map.as_mut().unwrap().extend(handles_hashmap);
}
