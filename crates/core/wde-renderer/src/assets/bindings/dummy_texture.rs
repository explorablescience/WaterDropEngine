use bevy::prelude::*;
use crate::prelude::*;

/// Placeholder texture used while real texture handles finish loading.
#[derive(Resource)]
pub struct DummyTexture(pub Handle<Texture>);
pub(crate) struct DummyTexturePlugin;
impl Plugin for DummyTexturePlugin {
    fn build(&self, _app: &mut App) {}
    fn finish(&self, app: &mut App) {
        // Load the dummy texture
        let assets_server = app.world().get_resource::<AssetServer>().unwrap();
        let dummy_texture = assets_server.load_with_settings("core/models/core/dummy_texture.png",
        |settings: &mut TextureLoaderSettings| {
            settings.label = "dummy-texture".to_string();
            settings.format = TextureFormat::R8Unorm;
            settings.usages = TextureUsages::TEXTURE_BINDING;
        });
        app.get_sub_app_mut(RenderApp).unwrap()
            .insert_resource(DummyTexture(dummy_texture));
    }
}
