use bevy::prelude::*;
use wde_renderer::prelude::*;

pub(crate) struct RenderTexturePlugin;
impl Plugin for RenderTexturePlugin {
    fn build(&self, app: &mut App) {
        let init_plugin = RenderBindingPluginRegister::<RenderTexture>::with_init(init, app);
        app.add_plugins(init_plugin).add_systems(Update, resize);
    }
}

/// The render texture for the pbr renderer.
///
/// Notes:
/// - It uses the [`SWAPCHAIN_FORMAT`], as well as the [`MSAA_SAMPLE_COUNT`] for multisampling.
/// - It is created and resized automatically based on the window size
/// - It is extracted and made available in the render world as a bind group for the lighting pass, so it can be sampled as an input texture.
/// - It is rendered to the swapchain in the final pass.
#[derive(Asset, Clone, TypePath, Default)]
pub struct RenderTexture(pub Handle<Texture>);
impl RenderBinding for RenderTexture {
    fn describe(&self, builder: &mut RenderBindingBuilder) {
        builder.add_texture_view(0, Some(self.0.clone()));
        builder.add_texture_sampler(1, Some(self.0.clone()));
    }

    fn label(&self) -> &str {
        "pbr-render-texture"
    }
}

fn init(mut commands: Commands, asset_server: Res<AssetServer>, window: Query<&Window>) {
    let resolution = &window.single().unwrap().resolution;
    let texture = asset_server.add(get_texture((
        resolution.physical_width(),
        resolution.physical_height()
    )));
    let texture = asset_server.add(RenderTexture(texture));
    commands.insert_resource(RenderBindingHolder(texture));
}

fn resize(
    mut commands: Commands,
    mut window_resized_events: MessageReader<SurfaceResized>,
    asset_server: Res<AssetServer>
) {
    for event in window_resized_events.read() {
        let texture = asset_server.add(get_texture((event.width, event.height)));
        let texture = asset_server.add(RenderTexture(texture));
        commands.insert_resource(RenderBindingHolder(texture));
    }
}

fn get_texture(size: (u32, u32)) -> Texture {
    Texture {
        label: "pbr-render-texture".to_string(),
        size,
        format: SWAPCHAIN_FORMAT,
        usages: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        sample_count: MSAA_SAMPLE_COUNT,
        ..Default::default()
    }
}
