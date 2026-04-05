//! Rendering system for the WDE renderer. Handle the initialization and presentation of the wgpu renderer.
use wde_logger::prelude::*;

use bevy::prelude::*;
use bevy::window::{PresentMode, PrimaryWindow, RawHandleWrapperHolder};
use wde_wgpu::instance::{self, PresentMode as WPresentMode, RenderEvent, setup_surface};

use crate::core::RenderInstance;

use super::SwapchainFrame;
use super::{EmptyWorld, extract_macros::ExtractWorld};

/// Initialize the main world empty resource.
pub(crate) fn init_main_world(mut commands: Commands) {
    // Initialize the empty world resource
    commands.init_resource::<EmptyWorld>();
}

/// Initialize the wgpu surface.
pub(crate) fn init_surface(
    mut commands: Commands,
    mut render_instance: ResMut<RenderInstance>,
    primary_window: ExtractWorld<Query<&RawHandleWrapperHolder, With<PrimaryWindow>>>,
    windows: ExtractWorld<Query<&Window>>
) {
    trace!("Initializing wgpu surface");

    // Create the wgpu surface
    let surface = {
        // Retrieve window
        let window_handle = primary_window
            .single()
            .unwrap()
            .0
            .lock()
            .expect("Couldn't get the window handle in time for surface initialization.");
        if let Some(wrapper) = window_handle.as_ref() {
            let handle = unsafe { wrapper.get_handle() };
            Some(
                render_instance
                    .as_ref()
                    .0
                    .read()
                    .unwrap()
                    .instance
                    .create_surface(handle)
                    .expect("Failed to create wgpu surface")
            )
        } else {
            error!("Failed to get window handle.");
            None
        }
    }
    .unwrap();

    // Store the surface configuration
    let surface_config = {
        let instance_ref = render_instance.as_ref().0.as_ref().read().unwrap();
        setup_surface(
            "wde_renderer",
            (600, 500),
            &instance_ref.device,
            &surface,
            &instance_ref.adapter,
            match windows.single().unwrap().present_mode {
                PresentMode::Fifo => WPresentMode::Fifo,
                PresentMode::FifoRelaxed => WPresentMode::FifoRelaxed,
                PresentMode::Mailbox => WPresentMode::Mailbox,
                PresentMode::Immediate => WPresentMode::Immediate,
                PresentMode::AutoVsync => WPresentMode::AutoVsync,
                PresentMode::AutoNoVsync => WPresentMode::AutoNoVsync
            }
        )
    };
    let mut mut_render_instance = render_instance.as_mut().0.write().unwrap();
    mut_render_instance.surface = Some(surface);
    mut_render_instance.surface_config = Some(surface_config);

    // Insert empty swapchain frame
    commands.init_resource::<SwapchainFrame>();
}

/// Prepare the rendering frame.
pub(crate) fn prepare(
    mut swapchain_frame: ResMut<SwapchainFrame>,
    render_instance: Res<RenderInstance>
) {
    let _ = debug_span!("prepare_swapchain_frame").entered();

    // Wait for the surface to be initialized
    if render_instance.0.read().unwrap().surface.is_none() {
        debug!("Waiting for surface to be initialized.");
        return;
    }

    // Retrieve the current texture
    let render_data = render_instance.0.read().unwrap();
    match instance::get_current_texture(
        render_data.surface.as_ref().unwrap(),
        render_data.surface_config.as_ref().unwrap()
    ) {
        RenderEvent::Redraw(render_texture) => {
            swapchain_frame.data = Some(render_texture);
        }
        RenderEvent::Resize => {
            let config = render_data.surface_config.as_ref().unwrap();

            // Check for zero size (minimized window)
            if config.width == 0 || config.height == 0 {
                debug!("Skipping resize: zero width/height.");
                return;
            }

            // Reconfigure the surface with updated dimensions
            info!(
                "Swapchain resize requested. Reconfiguring surface to {}x{}.",
                config.width, config.height
            );
            instance::resize(
                &render_data.device,
                render_data.surface.as_ref().unwrap(),
                config
            );

            // Retry immediately to get the next texture
            match instance::get_current_texture(
                render_data.surface.as_ref().unwrap(),
                render_data.surface_config.as_ref().unwrap()
            ) {
                RenderEvent::Redraw(render_texture) => {
                    swapchain_frame.data = Some(render_texture);
                }
                RenderEvent::Resize => {
                    // Return early - let the next frame acquire the texture after resize completes
                    // This is especially important on X11 where resize may not be synchronous
                    warn!("Surface resize still pending after reconfiguration.");
                }
                RenderEvent::None => {}
            }
        }
        RenderEvent::None => {}
    }
}

/// Present the rendered frame to the screen.
pub(crate) fn present(mut swapchain_frame: ResMut<SwapchainFrame>) {
    let _ = debug_span!("present_swapchain_frame").entered();

    let _ = instance::present(match swapchain_frame.data.take() {
        Some(render_texture) => render_texture.texture,
        None => {
            warn!("Skipping frame presentation: No swapchain render texture found.");
            return;
        }
    });
}
