use bevy::prelude::*;
use wde_renderer::{MSAA_SAMPLE_COUNT, prelude::*};

#[derive(Resource, Default)]
pub(crate) struct PbrDeferredTexturesLayout {
    pub deferred_layout: Option<BindGroupLayout>,
    pub deferred_bind_group: Option<BindGroup>,
    pub deferred_layout_resolved: Option<BindGroupLayout>,
    pub deferred_bind_group_resolved: Option<BindGroup>,
}
impl PbrDeferredTexturesLayout {
    /// Build the bind group for the deferred renderer.
    pub fn build_bind_group(
        textures: Res<RenderAssets<GpuTexture>>, render_instance: Res<RenderInstance>,
        mut textures_layout: ResMut<PbrDeferredTexturesLayout>, deferred_textures: Res<PbrDeferredTextures>
    ) {
        // Check if the bind group is already created
        if textures_layout.deferred_bind_group.is_some() & textures_layout.deferred_layout.is_some() {
            return;
        }

        // Get the textures
        let (depth, depth_resolved, albedo, albedo_resolved, normal, normal_resolved) = match (
            textures.get(&deferred_textures.depth),
            textures.get(&deferred_textures.depth_resolved),
            textures.get(&deferred_textures.albedo),
            textures.get(&deferred_textures.albedo_resolved),
            textures.get(&deferred_textures.normal),
            textures.get(&deferred_textures.normal_resolved)
        ) {
            (Some(depth), Some(depth_resolved), Some(albedo), Some(albedo_resolved), Some(normal), Some(normal_resolved)) =>
                (depth, depth_resolved, albedo, albedo_resolved, normal, normal_resolved),
            _ => return
        };

        // Create the layouts
        let deferred_layout = Self::layout();
        let deferred_layout_resolved = BindGroupLayout::new("deferred-textures-resolved", |builder: &mut BindGroupLayoutBuilder| {
            builder.add_texture_view(   0, ShaderStages::FRAGMENT, false);
            builder.add_texture_sampler(1, ShaderStages::FRAGMENT);
            builder.add_texture_view(   2, ShaderStages::FRAGMENT, false);
            builder.add_texture_sampler(3, ShaderStages::FRAGMENT);
            builder.add_texture_view(   4, ShaderStages::FRAGMENT, false);
            builder.add_texture_sampler(5, ShaderStages::FRAGMENT);
        });

        // Build the layout
        let render_instance = render_instance.0.read().unwrap();
        let deferred_layout_built = BindGroupLayout::build(&deferred_layout, &render_instance);
        let deferred_layout_resolved_built = BindGroupLayout::build(&deferred_layout_resolved, &render_instance);

        // Create the bind group
        let deferred_bind_group = BindGroupBuilder::build("deferred-textures", &render_instance, &deferred_layout_built, &vec![
            BindGroupBuilder::texture_view(   0, &depth.texture),
            BindGroupBuilder::texture_sampler(1, &depth.texture),
            BindGroupBuilder::texture_view(   2, &albedo.texture),
            BindGroupBuilder::texture_sampler(3, &albedo.texture),
            BindGroupBuilder::texture_view(   4, &normal.texture),
            BindGroupBuilder::texture_sampler(5, &normal.texture)
        ]);
        let deferred_bind_group_resolved = BindGroupBuilder::build("deferred-textures-resolved", &render_instance, &deferred_layout_resolved_built, &vec![
            BindGroupBuilder::texture_view(   0, &depth_resolved.texture),
            BindGroupBuilder::texture_sampler(1, &depth_resolved.texture),
            BindGroupBuilder::texture_view(   2, &albedo_resolved.texture),
            BindGroupBuilder::texture_sampler(3, &albedo_resolved.texture),
            BindGroupBuilder::texture_view(   4, &normal_resolved.texture),
            BindGroupBuilder::texture_sampler(5, &normal_resolved.texture)
        ]);

        // Insert the resources
        textures_layout.deferred_layout = Some(deferred_layout);
        textures_layout.deferred_bind_group = Some(deferred_bind_group);
        textures_layout.deferred_layout_resolved = Some(deferred_layout_resolved);
        textures_layout.deferred_bind_group_resolved = Some(deferred_bind_group_resolved);
    }

    pub fn layout() -> BindGroupLayout {
        BindGroupLayout::new("deferred-textures", |builder: &mut BindGroupLayoutBuilder| {
            builder.add_texture_view(   0, ShaderStages::FRAGMENT, true);
            builder.add_texture_sampler(1, ShaderStages::FRAGMENT);
            builder.add_texture_view(   2, ShaderStages::FRAGMENT, true);
            builder.add_texture_sampler(3, ShaderStages::FRAGMENT);
            builder.add_texture_view(   4, ShaderStages::FRAGMENT, true);
            builder.add_texture_sampler(5, ShaderStages::FRAGMENT);
        })
    }
}

#[derive(Resource)]
pub(crate) struct PbrDeferredTextures {
    pub depth: Handle<Texture>,
    pub depth_resolved: Handle<Texture>,
    pub albedo: Handle<Texture>,
    pub albedo_resolved: Handle<Texture>,
    pub normal: Handle<Texture>,
    pub normal_resolved: Handle<Texture>,
    pub resized: bool
}
impl PbrDeferredTextures {
    /// Create the textures for the deferred renderer.
    pub fn create_textures(mut commands: Commands, assets_server: Res<AssetServer>, window: Query<&Window>) {
        let resolution = &window.single().unwrap().resolution;

        // Create the depth texture
        let depth = assets_server.add(Texture {
            label: "pbr-depth".to_string(),
            size: (resolution.physical_width(), resolution.physical_height()),
            format: TextureFormat::R16Float,
            usages: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            sample_count: MSAA_SAMPLE_COUNT,
            ..Default::default()
        });
        let depth_resolved = assets_server.add(Texture {
            label: "pbr-depth-resolved".to_string(),
            size: (resolution.physical_width(), resolution.physical_height()),
            format: TextureFormat::R16Float,
            usages: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            ..Default::default()
        });

        // Create the albedo texture
        let albedo = assets_server.add(Texture {
            label: "pbr-albedo".to_string(),
            size: (resolution.physical_width(), resolution.physical_height()),
            format: TextureFormat::Rgba8UnormSrgb,
            usages: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            sample_count: MSAA_SAMPLE_COUNT,
            ..Default::default()
        });
        let albedo_resolved = assets_server.add(Texture {
            label: "pbr-albedo-resolved".to_string(),
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
            sample_count: MSAA_SAMPLE_COUNT,
            ..Default::default()
        });
        let normal_resolved = assets_server.add(Texture {
            label: "pbr-normal-resolved".to_string(),
            size: (resolution.physical_width(), resolution.physical_height()),
            format: TextureFormat::Rgba16Float,
            usages: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            ..Default::default()
        });

        // Insert the resources
        commands.insert_resource(PbrDeferredTextures {
            depth, depth_resolved, albedo, albedo_resolved, normal, normal_resolved, resized: false
        });
    }

    /// Resize the textures for the deferred renderer.
    pub fn resize_textures(
        mut window_resized_events: MessageReader<SurfaceResized>,
        server: Res<AssetServer>, mut deferred_textures: ResMut<PbrDeferredTextures>
    ) {
        deferred_textures.resized = false;
        for event in window_resized_events.read() {
            // Recreate the depth texture
            let depth = server.add(Texture {
                label: "pbr-depth".to_string(),
                size: (event.width, event.height),
                format: TextureFormat::R16Float,
                usages: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                sample_count: MSAA_SAMPLE_COUNT,
                ..Default::default()
            });
            let depth_resolved = server.add(Texture {
                label: "pbr-depth-resolved".to_string(),
                size: (event.width, event.height),
                format: TextureFormat::R16Float,
                usages: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                ..Default::default()
            });

            // Recreate the albedo texture
            let albedo = server.add(Texture {
                label: "pbr-albedo".to_string(),
                size: (event.width, event.height),
                format: TextureFormat::Rgba8UnormSrgb,
                usages: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                sample_count: MSAA_SAMPLE_COUNT,
                ..Default::default()
            });
            let albedo_resolved = server.add(Texture {
                label: "pbr-albedo-resolved".to_string(),
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
                sample_count: MSAA_SAMPLE_COUNT,
                ..Default::default()
            });
            let normal_resolved = server.add(Texture {
                label: "pbr-normal-resolved".to_string(),
                size: (event.width, event.height),
                format: TextureFormat::Rgba16Float,
                usages: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                ..Default::default()
            });

            // Insert the resources
            deferred_textures.depth = depth;
            deferred_textures.depth_resolved = depth_resolved;
            deferred_textures.albedo = albedo;
            deferred_textures.albedo_resolved = albedo_resolved;
            deferred_textures.normal = normal;
            deferred_textures.normal_resolved = normal_resolved;
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
            depth: textures.depth.clone(),
            depth_resolved: textures.depth_resolved.clone(),
            albedo: textures.albedo.clone(),
            albedo_resolved: textures.albedo_resolved.clone(),
            normal: textures.normal.clone(),
            normal_resolved: textures.normal_resolved.clone(),
            resized: false
        });
    }
}
