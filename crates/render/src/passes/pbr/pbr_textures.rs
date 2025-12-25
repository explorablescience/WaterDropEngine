use bevy::prelude::*;
use crate::{assets::{GpuTexture, RenderAssets, Texture}, core::{extract_macros::ExtractWorld, window::SurfaceResized}};
use wde_wgpu::{bind_group::{BindGroup, BindGroupLayout, BindGroupLayoutBuilder, WgpuBindGroup}, instance::WRenderInstance, render_pipeline::WShaderStages, texture::{WTextureFormat, WTextureUsages}};

#[derive(Resource, Default)]
pub struct PbrDeferredTexturesLayoutRegenerate(pub bool);

#[derive(Resource, Default)]
pub struct PbrDeferredTexturesLayout {
    pub deferred_layout: Option<BindGroupLayout>,
    pub deferred_bind_group: Option<WgpuBindGroup>
}
impl PbrDeferredTexturesLayout {
    /// Build the bind group for the deferred renderer.
    pub fn build_bind_group(
        textures: Res<RenderAssets<GpuTexture>>, render_instance: Res<WRenderInstance<'static>>,
        mut textures_layout: ResMut<PbrDeferredTexturesLayout>, deferred_textures: Res<PbrDeferredTextures>
    ) {
        // Check if the bind group is already created
        if textures_layout.deferred_bind_group.is_some() & textures_layout.deferred_layout.is_some() {
            return;
        }

        // Get the textures
        let (albedo, normal, material) = match (
            textures.get(&deferred_textures.albedo),
            textures.get(&deferred_textures.normal), textures.get(&deferred_textures.material)
        ) {
            (Some(albedo), Some(normal), Some(material)) =>
                (albedo, normal, material),
            _ => return
        };

        // Create the deferred layout
        let deferred_layout = BindGroupLayout::new("deferred-textures", |builder: &mut BindGroupLayoutBuilder| {
            builder.add_texture_view(   0, WShaderStages::FRAGMENT);
            builder.add_texture_sampler(1, WShaderStages::FRAGMENT);
            builder.add_texture_view(   2, WShaderStages::FRAGMENT);
            builder.add_texture_sampler(3, WShaderStages::FRAGMENT);
            builder.add_texture_view(   4, WShaderStages::FRAGMENT);
            builder.add_texture_sampler(5, WShaderStages::FRAGMENT);
        });

        // Build the layout
        let render_instance = render_instance.data.read().unwrap();
        let deferred_layout_built = BindGroupLayout::build(&deferred_layout, &render_instance);

        // Create the bind group
        let deferred_bind_group = BindGroup::build("deferred-textures", &render_instance, &deferred_layout_built, &vec![
            BindGroup::texture_view(   0, &albedo.texture),
            BindGroup::texture_sampler(1, &albedo.texture),
            BindGroup::texture_view(   2, &normal.texture),
            BindGroup::texture_sampler(3, &normal.texture),
            BindGroup::texture_view(   4, &material.texture),
            BindGroup::texture_sampler(5, &material.texture)
        ]);

        // Insert the resources
        textures_layout.deferred_layout = Some(deferred_layout);
        textures_layout.deferred_bind_group = Some(deferred_bind_group);
    }
}

#[derive(Resource)]
pub struct PbrDeferredTextures {
    pub albedo: Handle<Texture>,
    pub normal: Handle<Texture>,
    pub material: Handle<Texture>,
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
            format: WTextureFormat::Rgba8UnormSrgb,
            usages: WTextureUsages::RENDER_ATTACHMENT | WTextureUsages::TEXTURE_BINDING,
            ..Default::default()
        });

        // Create the normal texture
        let normal = assets_server.add(Texture {
            label: "pbr-normal".to_string(),
            size: (resolution.physical_width(), resolution.physical_height()),
            format: WTextureFormat::Rgba16Float,
            usages: WTextureUsages::RENDER_ATTACHMENT | WTextureUsages::TEXTURE_BINDING,
            ..Default::default()
        });

        // Create the material textures (metallic, roughness, reflectance)
        let material = assets_server.add(Texture {
            label: "pbr-material".to_string(),
            size: (resolution.physical_width(), resolution.physical_height()),
            format: WTextureFormat::Rgba8Unorm,
            usages: WTextureUsages::RENDER_ATTACHMENT | WTextureUsages::TEXTURE_BINDING,
            ..Default::default()
        });

        // Insert the resources
        commands.insert_resource(PbrDeferredTextures {
            albedo, normal, material, resized: false
        });
    }

    /// Resize the textures for the deferred renderer.
    pub fn resize_textures(
        mut window_resized_events: EventReader<SurfaceResized>,
        server: Res<AssetServer>, mut deferred_textures: ResMut<PbrDeferredTextures>
    ) {
        deferred_textures.resized = false;
        for event in window_resized_events.read() {
            // Recreate the albedo texture
            let albedo = server.add(Texture {
                label: "pbr-albedo".to_string(),
                size: (event.width, event.height),
                format: WTextureFormat::Rgba8UnormSrgb,
                usages: WTextureUsages::RENDER_ATTACHMENT | WTextureUsages::TEXTURE_BINDING,
                ..Default::default()
            });

            // Recreate the normal texture
            let normal = server.add(Texture {
                label: "pbr-normal".to_string(),
                size: (event.width, event.height),
                format: WTextureFormat::Rgba16Float,
                usages: WTextureUsages::RENDER_ATTACHMENT | WTextureUsages::TEXTURE_BINDING,
                ..Default::default()
            });

            // Recreate the material textures
            let material = server.add(Texture {
                label: "pbr-material".to_string(),
                size: (event.width, event.height),
                format: WTextureFormat::Rgba8Unorm,
                usages: WTextureUsages::RENDER_ATTACHMENT | WTextureUsages::TEXTURE_BINDING,
                ..Default::default()
            });

            // Insert the resources
            deferred_textures.albedo = albedo;
            deferred_textures.normal = normal;
            deferred_textures.material = material;
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
            material: textures.material.clone(),
            resized: false
        });
    }
}
