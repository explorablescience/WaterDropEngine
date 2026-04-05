use wde_logger::prelude::*;

use crate::{
    prelude::*,
    ui_textures::{UITextureHandle, UITextures}
};
use bevy::prelude::*;
use wde_egui::prelude::*;
use wde_renderer::prelude::*;

#[derive(Resource, Default)]
struct ScreenshotRequest {
    request: bool
}

#[derive(Resource, Default)]
struct RenderGraphUIState {
    pub names: Vec<String>,
    pub ghost_swapchain_texture_handle: Option<UITextureHandle>
}

pub struct RenderGraphPanelPlugin;
impl Plugin for RenderGraphPanelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ScreenshotRequest>()
            .add_systems(Startup, (init_ui, init_render))
            .add_systems(Update, (draw_ui, resize));

        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .init_resource::<RenderGraphUIState>()
            .init_resource::<ScreenshotRequest>()
            .add_systems(Extract, extract)
            .add_systems(Render, render_custom_passes.in_set(RenderSet::Render));
    }

    fn finish(&self, app: &mut App) {
        // Register the names of the render passes in the UI state resource, for display in the UI.
        let _render_graph = app
            .get_sub_app_mut(RenderApp)
            .unwrap()
            .world()
            .resource::<RenderGraph>();
        let names = vec![];
        // for id in render_graph.get_sorted_passes_OLD() {
        //     let pass = render_graph.get_pass_OLD(id).unwrap();
        //     names.push(format!("{}: {}", id, pass.label()));
        // }
        app.world_mut().insert_resource(RenderGraphUIState {
            names,
            ghost_swapchain_texture_handle: None
        });
    }
}

fn init_ui(mut ui_menu: ResMut<UIMenu>) {
    ui_menu.push("Engine/Render Graph");
}

fn init_render(
    asset_server: Res<AssetServer>,
    mut ui_textures: ResMut<UITextures>,
    mut render_graph_ui_state: ResMut<RenderGraphUIState>,
    window: Query<&Window>
) {
    // Create a texture used to blit from the swapchain, and register it in the UI textures resource
    let resolution = &window.single().unwrap().resolution;
    let texture = asset_server.add(Texture {
        label: "Swapchain Ghost Texture".to_string(),
        size: (resolution.physical_width(), resolution.physical_height()),
        format: SWAPCHAIN_FORMAT,
        usages: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
        ..Default::default()
    });
    let ghost_swapchain_texture_handle = ui_textures.register_texture(texture);

    // Insert the UI state resource
    render_graph_ui_state.ghost_swapchain_texture_handle = Some(ghost_swapchain_texture_handle);
}

fn resize(
    mut window_resized_events: MessageReader<SurfaceResized>,
    asset_server: Res<AssetServer>,
    mut ui_textures: ResMut<UITextures>,
    mut render_graph_ui_state: ResMut<RenderGraphUIState>
) {
    for event in window_resized_events.read() {
        // Recreate the ghost swapchain texture with the new size
        let texture = asset_server.add(Texture {
            label: "Swapchain Ghost Texture".to_string(),
            size: (event.width, event.height),
            format: SWAPCHAIN_FORMAT,
            usages: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
            ..Default::default()
        });
        let ghost_swapchain_texture_handle = ui_textures.register_texture(texture);

        // Update the UI state resource with the new texture handle
        render_graph_ui_state.ghost_swapchain_texture_handle = Some(ghost_swapchain_texture_handle);
    }
}

fn draw_ui(
    ctx: Res<EguiContext>,
    mut ui_menu: ResMut<UIMenu>,
    ui_state: Res<RenderGraphUIState>,
    mut render_messages: ResMut<ScreenshotRequest>,
    ui_textures: Res<UITextures>
) {
    if !ui_menu.is_clicked("Engine/Render Graph") {
        return;
    }

    *render_messages = ScreenshotRequest { request: false };
    egui::Window::new("Render Graph")
        .default_size(egui::vec2(400.0, 300.0))
        .open(
            ui_menu
                .clicked_mut("Engine/Render Graph")
                .unwrap_or(&mut false)
        )
        .show(&ctx.0, |ui| {
            ui.label("Registered Render Passes:");
            for name in &ui_state.names {
                ui.label(name);
            }
            ui.separator();
            if ui.button("Request Screenshot").clicked() {
                *render_messages = ScreenshotRequest { request: true };
            }

            // Display the ghost swapchain texture in the UI for debugging
            if let Some(ref handle) = ui_state.ghost_swapchain_texture_handle
                && let Some(swapchain_idx) = ui_textures.get_texture_index(handle.asset_id())
            {
                let ratio = 16.0 / 9.0;
                ui.image(egui::load::SizedTexture::new(
                    swapchain_idx,
                    egui::vec2(400.0, 400.0 / ratio)
                ));
            }
        });
}

fn extract(
    main_messages: ExtractWorld<Res<ScreenshotRequest>>,
    mut render_messages: ResMut<ScreenshotRequest>,
    main_render_graph_ui_state: ExtractWorld<Res<RenderGraphUIState>>,
    mut render_graph_ui_state: ResMut<RenderGraphUIState>
) {
    *render_messages = ScreenshotRequest {
        request: main_messages.request
    };
    render_graph_ui_state.ghost_swapchain_texture_handle = main_render_graph_ui_state
        .ghost_swapchain_texture_handle
        .clone();
}

fn render_custom_passes(world: &mut World) {
    // Check if a screenshot was requested
    if !world.resource::<ScreenshotRequest>().request {
        return;
    }
    info!("Capture of the render graph requested. Issuing render passes.");

    // Check if the ghost texture is available
    if world
        .resource::<RenderGraphUIState>()
        .ghost_swapchain_texture_handle
        .is_none()
    {
        warn!("No ghost swapchain texture handle available, skipping render passes.");
    }

    // Run the update methods for each pass
    // world.resource_scope(|world, graph: Mut<RenderGraph>| {
    //     let sorted_passes = graph.get_sorted_passes_OLD();
    //     for id in sorted_passes {
    //         // Get the pass and render it
    //         let pass = graph.get_pass_OLD(id).unwrap();
    //         let _span = debug_span!(
    //             "ui_screenshot_render_pass_render",
    //             pass_id = id,
    //             pass_name = pass.label()
    //         )
    //         .entered();
    //         pass.render(world);
    //         drop(_span);

    //         let _span = debug_span!(
    //             "ui_screenshot_render_pass_blit",
    //             pass_id = id,
    //             pass_name = pass.label()
    //         )
    //         .entered();
    //         // Get swapchain frame and its ghost texture
    //         let swapchain_frame = world.resource::<SwapchainFrame>().data.as_ref().unwrap();
    //         let ghost_texture_handle = world
    //             .resource::<RenderGraphUIState>()
    //             .ghost_swapchain_texture_handle
    //             .clone();
    //         let ghost_texture = world
    //             .resource::<RenderAssets<GpuTexture>>()
    //             .get(ghost_texture_handle.unwrap())
    //             .unwrap();

    //         // Blit the swapchain frame
    //         let render_instance = world.resource::<RenderInstance>().0.read().unwrap();
    //         let size = (
    //             render_instance.surface_config.as_ref().unwrap().width,
    //             render_instance.surface_config.as_ref().unwrap().height,
    //         );
    //         ghost_texture.texture.copy_from_surface_texture(
    //             &render_instance,
    //             &swapchain_frame.texture,
    //             size,
    //         );
    //         drop(_span);
    //     }
    // });
}
