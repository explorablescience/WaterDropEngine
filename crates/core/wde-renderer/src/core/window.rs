//! Window plugin and related components
//! This module contains the window plugin and related components.
//! It is responsible for creating and managing the window.

use bevy::{
    a11y::AccessibilityPlugin,
    app::{PluginGroup, PluginGroupBuilder},
    ecs::{message::Message, system::NonSendMarker},
    prelude::{Entity, Event, MessageReader, MessageWriter, Query, Res, ResMut, Resource, With},
    utils::default,
    window::{PresentMode, PrimaryWindow, Window, WindowPlugin, WindowResized, WindowTheme},
    winit::{WINIT_WINDOWS, WinitPlugin}
};
use wde_wgpu::instance;

use crate::core::RenderInstance;

use super::extract_macros::ExtractWorld;

/// Raw RGBA8 icon data used for the window icon shown in the OS taskbar/title bar.
/// Not supported on all platforms (e.g. Wayland ignores it; use a `.desktop` file there instead).
#[derive(Clone)]
pub struct WindowIcon {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32
}
impl WindowIcon {
    /// Decodes an encoded image (PNG, JPEG, ...) into an icon, for example from `include_bytes!`.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, image::ImageError> {
        let image = image::load_from_memory(bytes)?.into_rgba8();
        let (width, height) = image.dimensions();
        Ok(Self {
            rgba: image.into_raw(),
            width,
            height
        })
    }
}

/// Holds the icon to apply to the primary window once it has been created.
#[derive(Resource, Default)]
pub(crate) struct PrimaryWindowIcon(pub Option<WindowIcon>);

/// Applies the configured icon (if any) to the primary window.
/// Must run after the primary window has been created by winit (e.g. in `Startup`).
///
/// `WinitWindows` isn't stored as a regular non-send ECS resource in this bevy version (it lives in
/// a thread-local instead), so `NonSendMarker` is used to force this system onto the main thread,
/// which is the only thread the winit windows thread-local is populated on.
pub(crate) fn apply_window_icon(
    icon: Res<PrimaryWindowIcon>,
    _main_thread: NonSendMarker,
    primary_window: Query<Entity, With<PrimaryWindow>>
) {
    let Some(icon) = &icon.0 else { return };
    let Ok(entity) = primary_window.single() else {
        return;
    };

    WINIT_WINDOWS.with_borrow(|winit_windows| {
        let Some(winit_window) = winit_windows.get_window(entity) else {
            return;
        };

        match winit::window::Icon::from_rgba(icon.rgba.clone(), icon.width, icon.height) {
            Ok(winit_icon) => winit_window.set_window_icon(Some(winit_icon)),
            Err(err) => wde_logger::warn!("Failed to set window icon: {err}")
        }
    });
}

/// An event that is sent when the surface is resized.
/// This event is sent with the new width and height of the surface. It is used to update the surface configuration and resize the swap chain.
#[derive(Debug, Event, Message)]
pub struct SurfaceResized {
    pub width: u32,
    pub height: u32
}

pub(crate) struct WindowPlugins {
    pub title: String,
    pub resolution: (u32, u32)
}
impl Default for WindowPlugins {
    fn default() -> Self {
        Self {
            title: "WaterDropEngine".into(),
            resolution: (600, 500)
        }
    }
}
impl PluginGroup for WindowPlugins {
    fn build(self) -> PluginGroupBuilder {
        let mut group = PluginGroupBuilder::start::<Self>();

        // Add window and winit plugins
        group = group
            .add(WindowPlugin {
                primary_window: Some(Window {
                    title: self.title,
                    name: Some("waterdropengine".into()),
                    resolution: self.resolution.into(),
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
    mut events_writer: MessageWriter<SurfaceResized>,
    mut events_reader: MessageReader<WindowResized>,
    window: Query<&Window>
) {
    for _ in events_reader.read() {
        if let Ok(window) = window.single() {
            let (width, height) = (
                window.resolution.physical_width().max(1),
                window.resolution.physical_height().max(1)
            );

            // Check if window was minimized
            if width == 0 && height == 0 {
                continue;
            }

            // Send the surface resized event
            events_writer.write(SurfaceResized { width, height });
        }
    }
}

/// Extract the window size from the primary window and update the surface configuration.
pub(crate) fn extract_surface_size(
    render_instance: ResMut<RenderInstance>,
    windows: ExtractWorld<Query<&Window>>
) {
    // Check if there is a window
    if windows.iter().count() == 0 {
        return;
    }

    // Get the window size
    let window = windows.single().unwrap();
    let (width, height) = (
        window.resolution.physical_width().max(1),
        window.resolution.physical_height().max(1)
    );

    // Check if size different from old one
    let mut render_instance = render_instance.0.write().unwrap();
    let old_size = render_instance.surface_config.as_ref().unwrap();
    if width == old_size.width && height == old_size.height {
        return;
    }

    // Update the surface configuration
    let surface_config = render_instance.surface_config.as_mut().unwrap();
    surface_config.width = width;
    surface_config.height = height;

    instance::resize(
        &render_instance.device,
        render_instance.surface.as_ref().unwrap(),
        render_instance.surface_config.as_ref().unwrap()
    );
}
