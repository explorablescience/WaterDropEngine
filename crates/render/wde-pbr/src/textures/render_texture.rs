use bevy::prelude::*;
use wde_renderer::prelude::*;

#[derive(Resource, Default)]
pub struct RenderTextureBindGroup {
    pub bind_group: Option<BindGroup>
}
impl RenderTextureBindGroup {
    /// Build the bind group for the deferred renderer.
    pub fn build_bind_group(
        textures: Res<RenderAssets<GpuTexture>>,
        render_instance: Res<RenderInstance>,
        mut tex_bind_group: ResMut<RenderTextureBindGroup>,
        texture: Res<RenderTexture>
    ) {
        // Check if the bind group is already created
        if tex_bind_group.bind_group.is_some() {
            return;
        }

        // Get the texture
        let render_texture = match textures.get(&texture.texture) {
            Some(texture) => texture,
            None => return
        };

        // Build the layout
        let render_instance = render_instance.0.read().unwrap();
        let layout_built = BindGroupLayout::build(&Self::layout(), &render_instance).unwrap();

        // Create the bind group
        let bind_group = BindGroupBuilder::build(
            "pbr-render-texture",
            &render_instance,
            &layout_built,
            &vec![
                BindGroupBuilder::texture_view(0, &render_texture.texture),
                BindGroupBuilder::texture_sampler(1, &render_texture.texture),
            ]
        )
        .unwrap();

        // Insert the resources
        tex_bind_group.bind_group = Some(bind_group);
    }

    pub fn layout() -> BindGroupLayout {
        BindGroupLayout::new(
            "pbr-render-texture",
            |builder: &mut BindGroupLayoutBuilder| {
                builder.add_texture_view(0, ShaderStages::FRAGMENT, true);
                builder.add_texture_sampler(1, ShaderStages::FRAGMENT);
            }
        )
    }
}

/// The render texture for the pbr renderer.
/// This is where the lighting pass will render to.
/// This texture uses MSAA and is resolved later to the swapchain texture.
#[derive(Resource)]
pub struct RenderTexture {
    pub texture: Handle<Texture>,
    pub resized: bool
}
impl RenderTexture {
    pub fn create_texture(
        mut commands: Commands,
        assets_server: Res<AssetServer>,
        window: Query<&Window>
    ) {
        let resolution = &window.single().unwrap().resolution;
        let texture = assets_server.add(Texture {
            label: "pbr-render-texture".to_string(),
            size: (resolution.physical_width(), resolution.physical_height()),
            format: SWAPCHAIN_FORMAT,
            usages: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            sample_count: MSAA_SAMPLE_COUNT,
            ..Default::default()
        });
        commands.insert_resource(RenderTexture {
            texture,
            resized: false
        });
    }

    pub fn resize_texture(
        mut window_resized_events: MessageReader<SurfaceResized>,
        server: Res<AssetServer>,
        mut deferred_textures: ResMut<RenderTexture>
    ) {
        deferred_textures.resized = false;
        for event in window_resized_events.read() {
            let texture = server.add(Texture {
                label: "pbr-render-texture".to_string(),
                size: (event.width, event.height),
                format: SWAPCHAIN_FORMAT,
                usages: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                sample_count: MSAA_SAMPLE_COUNT,
                ..Default::default()
            });

            // Insert the resources
            deferred_textures.texture = texture;
            deferred_textures.resized = true;
        }
    }

    /// Extract the textures for the deferred renderer.
    pub fn extract_texture(
        mut commands: Commands,
        textures: ExtractWorld<Res<RenderTexture>>,
        mut textures_layout: ResMut<RenderTextureBindGroup>
    ) {
        if textures.resized {
            textures_layout.bind_group = None;
        }

        commands.insert_resource(RenderTexture {
            texture: textures.texture.clone(),
            resized: false
        });
    }
}
