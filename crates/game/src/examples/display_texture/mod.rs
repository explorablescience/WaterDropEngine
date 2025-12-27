use bevy::prelude::*;

mod pipeline;
mod pass;
mod material;

use wde_render::{assets::{MaterialsPluginRegister, RenderAssetsPlugin, Texture, TextureLoaderSettings}, core::RenderApp, passes::render_graph::RenderGraph};
use wde_wgpu::texture::WTextureFormat;

use crate::examples::display_texture::{material::{DisplayTextureMaterial, DisplayTextureMaterialAsset}, pass::{DisplayTexturePass, GPUMaterialHandler, RenderPassMesh}, pipeline::{DisplayTexturePipeline, DisplayTexturePipelineAsset, GpuDisplayTexturePipeline}};

pub struct DisplayTextureComponentPlugin;
impl Plugin for DisplayTextureComponentPlugin {
    fn build(&self, app: &mut App) {
        // Create the render pass mesh resource
        app
            .init_resource::<RenderPassMesh>()
            .add_systems(Startup, RenderPassMesh::init);
        app.get_sub_app_mut(RenderApp).unwrap()
            .init_resource::<RenderPassMesh>();

        // Register the material
        app.get_sub_app_mut(RenderApp).unwrap()
            .init_resource::<GPUMaterialHandler>();
        app.add_plugins(MaterialsPluginRegister::<DisplayTextureMaterialAsset>::default());
        app.register_type::<DisplayTextureMaterial>();

        // Add the pipeline asset
        app
            .init_asset::<DisplayTexturePipelineAsset>()
            .add_plugins(RenderAssetsPlugin::<GpuDisplayTexturePipeline>::default());

        // Add the render passes
        let mut render_graph = app.get_sub_app_mut(RenderApp).unwrap()
            .world_mut().get_resource_mut::<RenderGraph>().unwrap();
        render_graph.add_pass::<DisplayTexturePass>(2);

        // Load the entity on startup
        app.add_systems(Startup, load_entity);
    }

    fn finish(&self, app: &mut App) {
        // Create the pipeline
        let pipeline = app.world_mut()
            .get_resource::<AssetServer>().unwrap().add(DisplayTexturePipelineAsset);
        app.get_sub_app_mut(RenderApp).unwrap().world_mut().spawn(DisplayTexturePipeline(pipeline));
    }
}

pub fn load_entity(mut commands: Commands, server: Res<AssetServer>) {
    // Load the texture to display
    let texture: Handle<Texture> = server.load_with_settings("examples/display_texture/image.jpg", |settings: &mut TextureLoaderSettings| {
        settings.label = "display-texture".to_string();
        settings.format = WTextureFormat::Rgba8UnormSrgb;
    });

    // Create the material
    let material: Handle<DisplayTextureMaterialAsset> = server.add(DisplayTextureMaterialAsset {
        texture: Some(texture)
    });

    // Create the entity
    commands.spawn(DisplayTextureMaterial(material));
}
