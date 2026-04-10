mod deferred_textures;
mod depth;
mod render_texture;

pub use deferred_textures::*;
pub use depth::*;
pub use render_texture::*;

use bevy::prelude::*;
use wde_renderer::prelude::*;

pub(crate) struct CorePlugin;
impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            RenderTexturePlugin,
            DeferredTexturesPlugin,
            RenderDataRegisterPlugin::<DepthTexture>::default()
        ));
    }
}
