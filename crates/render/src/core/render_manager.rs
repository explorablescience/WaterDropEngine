//! Rendering system for the WDE renderer. Handle the initialization and presentation of the wgpu renderer.

use bevy::window::{PrimaryWindow, RawHandleWrapperHolder};
use bevy::prelude::*;
use wde_wgpu::instance::{self, setup_surface, WRenderEvent, WRenderInstance};

use super::SwapchainFrame;

use super::{extract_macros::ExtractWorld, EmptyWorld};

/// Initialize the main world empty resource.
pub(crate) fn init_main_world(mut commands: Commands) {
    // Initialize the empty world resource
    commands.init_resource::<EmptyWorld>();
}

/// Initialize the wgpu surface.
pub(crate) fn init_surface(mut commands: Commands, mut render_instance: ResMut<WRenderInstance<'static>>, primary_window: ExtractWorld<Query<&RawHandleWrapperHolder, With<PrimaryWindow>>>, windows: ExtractWorld<Query<&Window>>) {
    trace!("Initializing wgpu surface");
    
    // Create the wgpu surface
    let surface = {
        // Retrieve window
        let window_handle = primary_window.single().unwrap().0.lock().expect(
            "Couldn't get the window handle in time for surface initialization.",
        );
        if let Some(wrapper) = window_handle.as_ref() {
            let handle = unsafe { wrapper.get_handle() };
            Some(
                render_instance.as_ref().data.read().unwrap().instance
                    .create_surface(handle)
                    .expect("Failed to create wgpu surface"),
            )
        } else {
            error!("Failed to get window handle.");
            None
        }
    }.unwrap();

    // Store the surface configuration
    let surface_config = {
        let instance_ref = render_instance.as_ref().data.as_ref().read().unwrap();
        setup_surface("wde_renderer", (600, 500),
            &instance_ref.device, &surface, &instance_ref.adapter, windows.single().unwrap().present_mode)
    };
    let mut mut_render_instance = render_instance.as_mut().data.write().unwrap();
    mut_render_instance.surface = Some(surface);
    mut_render_instance.surface_config = Some(surface_config);

    // Insert empty swapchain frame
    commands.init_resource::<SwapchainFrame>();
}

/// Prepare the rendering frame.
pub(crate) fn prepare(mut swapchain_frame: ResMut<SwapchainFrame>, render_instance: Res<WRenderInstance<'static>>) {
    // Wait for the surface to be initialized
    if render_instance.data.read().unwrap().surface.is_none() {
        debug!("Waiting for surface to be initialized.");
        return
    }
    
    // Retrieve the current texture
    let render_data = render_instance.data.read().unwrap();
    match instance::get_current_texture(
        render_data.surface.as_ref().unwrap(), 
        render_data.surface_config.as_ref().unwrap()
    ) {
        WRenderEvent::Redraw(render_texture) => {
            swapchain_frame.data = Some(render_texture);
        },
        WRenderEvent::Resize => {
            debug!("Received resize event from render instance.");
            instance::resize(&render_data.device, render_data.surface.as_ref().unwrap(),
                render_data.surface_config.as_ref().unwrap());
            match instance::get_current_texture(
                render_data.surface.as_ref().unwrap(),
                render_data.surface_config.as_ref().unwrap()
            ) {
                WRenderEvent::Redraw(render_texture) => {
                    swapchain_frame.data = Some(render_texture);
                },
                _ => {
                    error!("Failed to get current texture after resize event.");
                },
            }
        },
        WRenderEvent::None => {},
    }
}

/// Present the rendered frame to the screen.
pub(crate) fn present(mut swapchain_frame: ResMut<SwapchainFrame>) {
    let _ = instance::present(match swapchain_frame.data.take() {
        Some(render_texture) => render_texture.texture,
        None => {
            error!("Failed to present frame: no render texture found.");
            return
        },
    });
}
