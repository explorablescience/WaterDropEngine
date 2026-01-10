use bevy::prelude::*;
use wde_renderer::prelude::*;

#[derive(Resource, Default)]
pub(crate) struct PbrDeferredTexturesLayout {
    pub deferred_layout: Option<BindGroupLayout>,
    pub deferred_bind_group: Option<BindGroup>
}
impl PbrDeferredTexturesLayout {
    /// Build the bind group for the deferred renderer.
    pub fn build_bind_group(
        textures: Res<RenderAssets<GpuTexture>>, render_instance: Res<RenderInstance<'static>>,
        mut textures_layout: ResMut<PbrDeferredTexturesLayout>, deferred_textures: Res<PbrDeferredTextures>
    ) {
        // Check if the bind group is already created
        if textures_layout.deferred_bind_group.is_some() & textures_layout.deferred_layout.is_some() {
            return;
        }

        // Get the textures
        let (albedo, normal) = match (
            textures.get(&deferred_textures.albedo),
            textures.get(&deferred_textures.normal)
        ) {
            (Some(albedo), Some(normal)) =>
                (albedo, normal),
            _ => return
        };

        // Create the deferred layout
        let deferred_layout = BindGroupLayout::new("deferred-textures", |builder: &mut BindGroupLayoutBuilder| {
            builder.add_texture_view(   0, ShaderStages::FRAGMENT);
            builder.add_texture_sampler(1, ShaderStages::FRAGMENT);
            builder.add_texture_view(   2, ShaderStages::FRAGMENT);
            builder.add_texture_sampler(3, ShaderStages::FRAGMENT);
        });

        // Build the layout
        let render_instance = render_instance.0.read().unwrap();
        let deferred_layout_built = BindGroupLayout::build(&deferred_layout, &render_instance);

        // Create the bind group
        let deferred_bind_group = BindGroupBuilder::build("deferred-textures", &render_instance, &deferred_layout_built, &vec![
            BindGroupBuilder::texture_view(   0, &albedo.texture),
            BindGroupBuilder::texture_sampler(1, &albedo.texture),
            BindGroupBuilder::texture_view(   2, &normal.texture),
            BindGroupBuilder::texture_sampler(3, &normal.texture)
        ]);

        // Insert the resources
        textures_layout.deferred_layout = Some(deferred_layout);
        textures_layout.deferred_bind_group = Some(deferred_bind_group);
    }
}

#[derive(Resource)]
pub(crate) struct PbrDeferredTextures {
    pub albedo: Handle<Texture>,
    pub normal: Handle<Texture>,
    pub resized: bool
}
impl PbrDeferredTextures {
    /// Create the textures for the deferred renderer.
    pub fn create_textures(mut commands: Commands, assets_server: Res<AssetServer>, window: Query<&Window>) {
        let resolution = &window.single().unwrap().resolution;

        // Create the albedo texture
        let albedo = assets_server.add(Texture {
            label: "pbr-albedo".to_string(),
            size: (resolution.physical_width(), resolution.physical_height()),
            format: TextureFormat::Rgba8UnormSrgb,
            usages: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            ..Default::default()
        });

        // Create the normal texture
        let normal = assets_server.add(Texture {
            label: "pbr-normal".to_string(),
            size: (resolution.physical_width(), resolution.physical_height()),
            format: TextureFormat::Rgba16Float,
            usages: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            ..Default::default()
        });

        // Insert the resources
        commands.insert_resource(PbrDeferredTextures {
            albedo, normal, resized: false
        });
    }

    /// Resize the textures for the deferred renderer.
    pub fn resize_textures(
        mut window_resized_events: MessageReader<SurfaceResized>,
        server: Res<AssetServer>, mut deferred_textures: ResMut<PbrDeferredTextures>
    ) {
        deferred_textures.resized = false;
        for event in window_resized_events.read() {
            // Recreate the albedo texture
            let albedo = server.add(Texture {
                label: "pbr-albedo".to_string(),
                size: (event.width, event.height),
                format: TextureFormat::Rgba8UnormSrgb,
                usages: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                ..Default::default()
            });

            // Recreate the normal texture
            let normal = server.add(Texture {
                label: "pbr-normal".to_string(),
                size: (event.width, event.height),
                format: TextureFormat::Rgba16Float,
                usages: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                ..Default::default()
            });

            // Insert the resources
            deferred_textures.albedo = albedo;
            deferred_textures.normal = normal;
            deferred_textures.resized = true;
        }
    }

    /// Extract the textures for the deferred renderer.
    pub fn extract_textures(mut commands: Commands, textures: ExtractWorld<Res<PbrDeferredTextures>>, mut textures_layout: ResMut<PbrDeferredTexturesLayout>) {
        if textures.resized {
            textures_layout.deferred_layout = None;
            textures_layout.deferred_bind_group = None;
        }

        commands.insert_resource(PbrDeferredTextures {
            albedo: textures.albedo.clone(),
            normal: textures.normal.clone(),
            resized: false
        });
    }
}
