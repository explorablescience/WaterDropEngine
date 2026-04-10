use bevy::prelude::*;
use wde_renderer::prelude::*;

pub(crate) struct DepthTexturePlugin;
impl Plugin for DepthTexturePlugin {
    fn build(&self, app: &mut App) {
        let init_plugin = RenderBindingPluginRegisterOld::<DepthTexture>::with_init(init, app);
        app.add_plugins(init_plugin).add_systems(Update, resize);
    }
}

/// Describes a depth texture that can be used in rendering.
///
/// Notes:
/// - It uses the [`DEPTH_FORMAT`], as well as the [`MSAA_SAMPLE_COUNT`] for multisampling.
/// - It is created and resized automatically based on the window size
#[derive(Asset, Clone, TypePath, Default)]
pub struct DepthTexture(pub Handle<Texture>);
impl RenderBindingOld for DepthTexture {
    fn describe(&self, builder: &mut RenderBindingBuilderOld) {
        builder.add_texture_view(0, Some(self.0.clone()));
        builder.add_texture_sampler(1, Some(self.0.clone()));
    }
}

fn init(mut commands: Commands, asset_server: Res<AssetServer>, window: Query<&Window>) {
    let resolution = &window.single().unwrap().resolution;
    let texture = asset_server.add(get_texture((
        resolution.physical_width(),
        resolution.physical_height()
    )));
    let texture = asset_server.add(DepthTexture(texture));
    commands.insert_resource(RenderBindingHolderOld(texture));
}

fn resize(
    mut commands: Commands,
    mut window_resized_events: MessageReader<SurfaceResized>,
    asset_server: Res<AssetServer>
) {
    for event in window_resized_events.read() {
        let texture = asset_server.add(get_texture((event.width, event.height)));
        let texture = asset_server.add(DepthTexture(texture));
        commands.insert_resource(RenderBindingHolderOld(texture));
    }
}

fn get_texture(size: (u32, u32)) -> Texture {
    Texture {
        label: "depth".to_string(),
        size,
        format: DEPTH_FORMAT,
        usages: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        sample_count: MSAA_SAMPLE_COUNT,
        ..Default::default()
    }
}
