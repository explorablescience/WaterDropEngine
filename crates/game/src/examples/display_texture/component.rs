use bevy::prelude::*;
use wde_render::assets::{Texture, TextureLoaderSettings};
use wde_wgpu::texture::WTextureFormat;

pub struct DisplayTextureComponentPlugin;
impl Plugin for DisplayTextureComponentPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, load_texture)
            .add_systems(Update, delete_entity_after_time)
            .init_resource::<Counter>();
    }
}

#[derive(Component, Default)]
pub struct DisplayTextureComponent {
    pub texture: Handle<Texture>,
}

fn load_texture(mut commands: Commands, server: Res<AssetServer>) {
    // Load the texture to display
    let texture: Handle<Texture> = server.load_with_settings("examples/display_texture/texture.png", |settings: &mut TextureLoaderSettings| {
        settings.label = "display-texture".to_string();
        settings.format = WTextureFormat::R8Unorm; 
    });

    // Create the terrain entity
    commands.spawn(DisplayTextureComponent { texture });
}

#[derive(Resource, Default)]
pub struct Counter(u32);

fn delete_entity_after_time(
    mut commands: Commands, display_texture_components: Query<Entity, With<DisplayTextureComponent>>,
    mut counter: ResMut<Counter>
) {
    if counter.0 == 3000 {
        // Despawn the entity
        // This should unload the texture as no other entity is using it
        let entity = display_texture_components.single().unwrap();
        commands.entity(entity).despawn();
    }
    counter.0 += 1;
}
