
use wde_egui::prelude::*;
use wde_logger::prelude::*;
use bevy::prelude::*;
use wde_renderer::prelude::*;

pub struct UITexturesPlugin;
impl Plugin for UITexturesPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(UITextures { handles: Some(vec![]), textures_to_extract: Some(vec![]) });
        app.get_sub_app_mut(RenderApp).unwrap()
            .insert_resource(UITextures { handles: Some(vec![]), textures_to_extract: None })
            .add_systems(Render, UITextures::update_images.in_set(RenderSet::Prepare));
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
    handles: Option<Vec<UITextureHandle>>,
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

    fn update_images(
        mut egui_render_pass: ResMut<EguiRenderPass>,
        render_instance: Res<RenderInstance>,
    ) {
        // Todo
        // let renderer = egui_render_pass.renderer.as_ref().unwrap().write().unwrap();
        // let render_instance = render_instance.0.read().unwrap();
        // renderer.update_egui_texture_from_wgpu_texture(&render_instance.device, texture, texture_filter, id);
    }
}
