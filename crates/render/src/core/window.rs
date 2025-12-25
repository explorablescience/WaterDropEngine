//! Window plugin and related components
//! 
//! This module contains the window plugin and related components.
//! It is responsible for creating and managing the window.

use bevy::{a11y::AccessibilityPlugin, app::{PluginGroup, PluginGroupBuilder}, ecs::message::Message, prelude::{Event, EventReader, EventWriter, Query, ResMut}, utils::default, window::{PresentMode, Window, WindowPlugin, WindowResized, WindowTheme}, winit::WinitPlugin};
use wde_wgpu::instance::WRenderInstance;

use super::extract_macros::ExtractWorld;

/// An event that is sent when the surface is resized.
#[derive(Debug, Event, Message)]
pub struct SurfaceResized {
    pub width: u32,
    pub height: u32,
}

pub(crate) struct WindowPlugins;

impl PluginGroup for WindowPlugins {
    fn build(self) -> PluginGroupBuilder {
        let mut group = PluginGroupBuilder::start::<Self>();

        // Add window and winit plugins
        group = group
            .add(WindowPlugin {
                primary_window: Some(Window {
                    title: "WaterDropEngine".into(),
                    name: Some("waterdropengine".into()),
                    resolution: (600, 500).into(),
                    present_mode: PresentMode::AutoVsync,
                    fit_canvas_to_parent: true,
                    prevent_default_event_handling: false,
                    window_theme: Some(WindowTheme::Dark),
                    enabled_buttons: bevy::window::EnabledButtons {
                        maximize: true,
                        ..Default::default()
                    },
                    visible: true,
                    ..default()
                }),
                ..default()
            })
            .add::<WinitPlugin>(WinitPlugin::default())
            .add(AccessibilityPlugin);

        group
    }
}


/// Send surface resized events with the physical window size.
pub(crate) fn send_surface_resized(
    mut events_writer: EventWriter<SurfaceResized>, 
    mut events_reader: EventReader<WindowResized>, window: Query<&Window>
) {
    for _ in events_reader.read() {
        if let Ok(window) = window.single() {
            let (width, height) = (
                window.resolution.physical_width().max(1),
                window.resolution.physical_height().max(1),
            );

            // Check if window was minimized
            if width == 0 && height == 0 {
                continue
            }

            // Send the surface resized event
            events_writer.write(SurfaceResized { width, height });
        }
    }
}


/// Extract the window size from the primary window and update the surface configuration.
pub(crate) fn extract_surface_size(render_instance: ResMut<WRenderInstance<'static>>, windows: ExtractWorld<Query<&Window>>) {
    // Check if there is a window
    if windows.iter().count() == 0 {
        return
    }

    // Get the window size
    let window = windows.single().unwrap();
    let (width, height) = (
        window.resolution.physical_width().max(1),
        window.resolution.physical_height().max(1),
    );
    
    // Update the surface configuration
    let mut render_instance = render_instance.data.write().unwrap();
    let surface_config = render_instance.surface_config.as_mut().unwrap();
    surface_config.width = width;
    surface_config.height = height;
}
