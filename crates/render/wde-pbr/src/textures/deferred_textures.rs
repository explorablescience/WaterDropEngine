use bevy::prelude::*;
use wde_renderer::prelude::*;

pub(crate) struct DeferredTexturesPlugin;
impl Plugin for DeferredTexturesPlugin {
    fn build(&self, app: &mut App) {
        let init_plugin = RenderBindingPluginRegister::<DeferredTextures>::with_init(init, app);
        app.add_plugins(init_plugin).add_systems(Update, resize);

        let init_resolved_plugin =
            RenderBindingPluginRegister::<DeferredTexturesResolved>::with_init(init_resolved, app);
        app.add_plugins(init_resolved_plugin)
            .add_systems(Update, resize);
    }
}

#[derive(Asset, Clone, TypePath, Default)]
pub struct DeferredTextures {
    pub depth: Handle<Texture>,
    pub albedo: Handle<Texture>,
    pub normal: Handle<Texture>
}
impl DeferredTextures {
    pub const DEPTH_BINDING: u32 = 0;
    pub const ALBEDO_BINDING: u32 = 2;
    pub const NORMAL_BINDING: u32 = 4;
}
impl RenderBinding for DeferredTextures {
    fn describe(&self, builder: &mut RenderBindingBuilder) {
        builder.add_texture_view(DeferredTextures::DEPTH_BINDING, Some(self.depth.clone()));
        builder.add_texture_sampler(
            DeferredTextures::DEPTH_BINDING + 1,
            Some(self.depth.clone())
        );
        builder.add_texture_view(DeferredTextures::ALBEDO_BINDING, Some(self.albedo.clone()));
        builder.add_texture_sampler(
            DeferredTextures::ALBEDO_BINDING + 1,
            Some(self.albedo.clone())
        );
        builder.add_texture_view(DeferredTextures::NORMAL_BINDING, Some(self.normal.clone()));
        builder.add_texture_sampler(
            DeferredTextures::NORMAL_BINDING + 1,
            Some(self.normal.clone())
        );
    }

    fn label(&self) -> &str {
        "pbr-deferred-textures"
    }
}

#[derive(Asset, Clone, TypePath, Default)]
pub struct DeferredTexturesResolved {
    pub depth: Handle<Texture>,
    pub albedo: Handle<Texture>,
    pub normal: Handle<Texture>
}
impl DeferredTexturesResolved {
    pub const DEPTH_BINDING: u32 = 0;
    pub const ALBEDO_BINDING: u32 = 2;
    pub const NORMAL_BINDING: u32 = 4;
}
impl RenderBinding for DeferredTexturesResolved {
    fn describe(&self, builder: &mut RenderBindingBuilder) {
        builder.add_texture_view(
            DeferredTexturesResolved::DEPTH_BINDING,
            Some(self.depth.clone())
        );
        builder.add_texture_sampler(
            DeferredTexturesResolved::DEPTH_BINDING + 1,
            Some(self.depth.clone())
        );
        builder.add_texture_view(
            DeferredTexturesResolved::ALBEDO_BINDING,
            Some(self.albedo.clone())
        );
        builder.add_texture_sampler(
            DeferredTexturesResolved::ALBEDO_BINDING + 1,
            Some(self.albedo.clone())
        );
        builder.add_texture_view(
            DeferredTexturesResolved::NORMAL_BINDING,
            Some(self.normal.clone())
        );
        builder.add_texture_sampler(
            DeferredTexturesResolved::NORMAL_BINDING + 1,
            Some(self.normal.clone())
        );
    }

    fn label(&self) -> &str {
        "pbr-deferred-textures-resolved"
    }
}

fn init(mut commands: Commands, asset_server: Res<AssetServer>, window: Query<&Window>) {
    let resolution = &window.single().unwrap().resolution;

    // Create the textures
    let (depth_texture, albedo_texture, normal_texture) = get_textures(
        (resolution.physical_width(), resolution.physical_height()),
        false
    );
    let deferred_textures = asset_server.add(DeferredTextures {
        depth: asset_server.add(depth_texture),
        albedo: asset_server.add(albedo_texture),
        normal: asset_server.add(normal_texture)
    });
    commands.insert_resource(RenderBindingHolder(deferred_textures));
}

fn init_resolved(mut commands: Commands, asset_server: Res<AssetServer>, window: Query<&Window>) {
    let resolution = &window.single().unwrap().resolution;

    // Create the textures
    let (depth_texture, albedo_texture, normal_texture) = get_textures(
        (resolution.physical_width(), resolution.physical_height()),
        true
    );
    let resolved_textures = asset_server.add(DeferredTexturesResolved {
        depth: asset_server.add(depth_texture),
        albedo: asset_server.add(albedo_texture),
        normal: asset_server.add(normal_texture)
    });
    commands.insert_resource(RenderBindingHolder(resolved_textures));
}

fn resize(
    mut commands: Commands,
    mut window_resized_events: MessageReader<SurfaceResized>,
    asset_server: Res<AssetServer>
) {
    for event in window_resized_events.read() {
        // Recreate the textures with the new window size
        let (depth_texture, albedo_texture, normal_texture) =
            get_textures((event.width, event.height), false);
        let deferred_textures = asset_server.add(DeferredTextures {
            depth: asset_server.add(depth_texture),
            albedo: asset_server.add(albedo_texture),
            normal: asset_server.add(normal_texture)
        });
        commands.insert_resource(RenderBindingHolder(deferred_textures));

        let (depth_texture, albedo_texture, normal_texture) =
            get_textures((event.width, event.height), true);
        let resolved_textures = asset_server.add(DeferredTexturesResolved {
            depth: asset_server.add(depth_texture),
            albedo: asset_server.add(albedo_texture),
            normal: asset_server.add(normal_texture)
        });
        commands.insert_resource(RenderBindingHolder(resolved_textures));
    }
}

fn get_textures(size: (u32, u32), is_resolved: bool) -> (Texture, Texture, Texture) {
    let sample_count = if is_resolved { 1 } else { MSAA_SAMPLE_COUNT };
    let depth_texture = Texture {
        label: format!("pbr-depth{}", if is_resolved { "-resolved" } else { "" }),
        size,
        format: TextureFormat::R16Float,
        usages: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        sample_count,
        ..Default::default()
    };
    let albedo_texture = Texture {
        label: format!("pbr-albedo{}", if is_resolved { "-resolved" } else { "" }),
        size,
        format: TextureFormat::Rgba8UnormSrgb,
        usages: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        sample_count,
        ..Default::default()
    };
    let normal_texture = Texture {
        label: format!("pbr-normal{}", if is_resolved { "-resolved" } else { "" }),
        size,
        format: TextureFormat::Rgba16Float,
        usages: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        sample_count,
        ..Default::default()
    };
    (depth_texture, albedo_texture, normal_texture)
}
