use bevy::prelude::*;

mod display_texture;
mod pbr_batches;
mod custom_forward_render;

#[allow(dead_code)]
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Examples {
    /// No example selected
    None,
    /// Display a texture onto the screen view
    DisplayTexture,
    /// Create a scene with pbr batches of entities with different pbr materials and meshes
    PbrBatches,
    /// Implementation of a custom forward render pass, pipeline and material
    CustomForwardRender
}

pub static mut SELECTED_EXAMPLE: Examples = Examples::DisplayTexture;

pub struct ExamplesPugin;
impl Plugin for ExamplesPugin {
    fn build(&self, app: &mut App) {
        // Load the selected example
        unsafe {
            match SELECTED_EXAMPLE {
                Examples::None => {}
                Examples::DisplayTexture => {
                    app.add_plugins(display_texture::DisplayTextureComponentPlugin);
                },
                Examples::PbrBatches => {
                    app.add_plugins(pbr_batches::PbrBatchesPlugin);
                },
                Examples::CustomForwardRender => {
                    app.add_plugins(custom_forward_render::CustomFeaturesPlugin);
                }
            }
        }
    }
}
