use bevy::prelude::*;
use wde::prelude::*;

#[derive(Asset, Clone, TypePath)]
/// Describes a simple material with a texture.
pub struct DisplayTextureMaterialAsset {
    pub texture: Option<Handle<Texture>>,
}

/// Describes a simple material with a texture.
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct DisplayTextureMaterial(pub Handle<DisplayTextureMaterialAsset>);
impl RenderBinding for DisplayTextureMaterialAsset {
    fn describe(&self, builder: &mut RenderBindingBuilder) {
        // builder.add_texture_view(    0, ShaderStages::FRAGMENT, self.texture.clone());
        // builder.add_texture_sampler( 1, ShaderStages::FRAGMENT, self.texture.clone());
    }

    fn label(&self) -> &str {
        "display-texture-material"
    }
}
