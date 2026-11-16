//! The renderer module is responsible for rendering the scene.
//!
//! It extracts the main world into the render world and runs the render schedule.
//! It provides the [`Render`] schedule of the [`RenderApp`], whose main world extraction runs in the [`ExtractSchedule`] (see [`RenderSet`] for the sets of the render schedule).
//! It also provides multiple resources, such as the [`RenderInstance`] and the [`SwapchainFrame`], which are used by the render graph and the render pipelines.
//! Lastly, it also handles window events, such as resizing, and sends the corresponding events to the render app.
//!
//!
//! # Render resources
//! - The render instance is stored in the [`RenderInstance`] resource (available in the render app), which is an Arc<RwLock<>> to allow parallelism in the render app.
//! - The current swap chain frame is stored in the [`SwapchainFrame`] resource (available in the render app).
//! - The device limits are stored in the [`DeviceLimits`] resource (available in both the main app and the render app).
//!
//!
//! # Simple render system
//! To add a simple render system, you can add a system to the render schedule. For example, to update the camera buffer before rendering, you can add a system to the `RenderSet::Prepare` set of the render schedule:
//! ```
//! app.get_sub_app_mut(RenderApp).unwrap()
//!     .add_systems(Render, update_camera_buffer.in_set(RenderSet::Prepare));
//! ```
//!
//!
//! # Extract phase
//! ## Extracting data from the main world
//! As the render app runs in a separate thread from the main app, it cannot access the main world directly. To extract data from the main world, you can use the [`ExtractSchedule`] and the [`Extract`](crate::sync::Extract) system parameter. For example, to extract the camera data from the main world, you can add a system to the extract schedule:
//! ```
//! app.get_sub_app_mut(RenderApp).unwrap()
//!     .add_systems(ExtractSchedule, extract_camera_data);
//!
//! fn extract_camera_data(mut commands: Commands, camera_query: Extract<Query<&Camera>>) {
//!     let camera = camera_query.single();
//!     commands.insert_resource(ExtractedCameraData {
//!         // ...
//!     });
//! }
//! ```
//!
//! ## Extracting while mutating the main world
//! If you need to extract data from the main world while also mutating it, you can use the [`MainWorld`](crate::sync::MainWorld) resource:
//! ```
//! pub fn extract_messages(
//!   mut render_test_resource: ResMut<TestResource>,
//!   mut main_world: ResMut<MainWorld>
//! ) {
//!    /* (...) */
//! }
//! ```
//!
//! ## Extracting resources and entities
//! To extract resources and entities from the main world, see the [`sync`](crate::sync) module, which provides utilities to automatically extract resources, query and entities from the main to the render world.
//!
//! # Window events
//! The renderer also handles window events, such as resizing. If the window is resized, an event of type [`SurfaceResized`] is sent to the main and render app, which contains the new width and height of the window in physical pixels.
//! To listen to these events, you can do:
//! ```
//! app.get_sub_app_mut(RenderApp).unwrap()
//!     .add_systems(Render, handle_resize_events);
//!
//! fn handle_resize_events(mut window_resized_events: MessageReader<SurfaceResized>) {
//!     for event in window_resized_events.read() {
//!         // Handle the resize event
//!     }
//! }
//! ```

mod render_manager;
mod render_multithread;
mod window;

pub use window::{FullscreenKey, SurfaceResized, WindowIcon, WindowShortcutSet};

use bevy::{
    app::AppLabel,
    ecs::{schedule::ScheduleLabel, system::SystemState},
    extract::{ExtractPlugin, ExtractSchedule},
    prelude::*,
    tasks::futures_lite,
    window::{PrimaryWindow, RawHandleWrapperHolder}
};
use render_manager::{init_surface, prepare, present};
use render_multithread::PipelinedRenderingPlugin;
use std::sync::{Arc, RwLock};
use wde_wgpu::instance::{Limits, RenderTexture, create_instance};
use window::{
    PrimaryWindowIcon, WindowPlugins, apply_window_icon, extract_surface_size,
    send_surface_resized, toggle_fullscreen
};

use crate::passes::{PipelineManagerPlugin, RenderGraph};

/// The renderer schedule set.
/// The render schedule will be executed by the renderer app.
#[derive(SystemSet, Hash, PartialEq, Eq, Clone, Copy, Debug)]
pub enum RenderSet {
    /// Run the extract commands registered during the extract schedule. This set is executed automatically and should not be used directly. Instead, use the [`ExtractSchedule`].
    ExtractAuto,
    /// Prepare resources before rendering. This includes updating buffers, textures, assets, bind groups, etc.
    Prepare,
    /// Render commands.
    Render,
    /// Submit commands.
    Submit,
    /// Despawn the temporary render world entities.
    Cleanup
}

/// The renderer schedule.
/// This schedule is responsible for rendering the scene.
#[derive(ScheduleLabel, Hash, PartialEq, Eq, Clone, Copy, Debug)]
pub struct Render;
impl Render {
    pub fn base() -> Schedule {
        use RenderSet::*;

        let mut schedule = Schedule::new(Self);
        schedule.configure_sets((ExtractAuto, Prepare, Render, Submit, Cleanup).chain());

        schedule
    }
}

/// The render app. This is the app that is responsible for rendering the scene. It runs in a separate thread from the main app and has its own schedule and resources. It is used to extract the main world into the render world and run the render schedule. It is also used to manage the swap chain and present the rendered frame to the window.
#[derive(Debug, Clone, Copy, Default, Hash, PartialEq, Eq, AppLabel)]
pub struct RenderApp;

/// The wgpu render instance resource.
/// It is wrapped in an Arc<RwLock<>> to allow it to be shared between the main app and the render app, and to allow it to be mutated from both apps without borrowing issues. It is also wrapped in a resource to avoid borrowing issues when accessing it from different systems.
#[derive(Resource)]
pub struct RenderInstance(pub Arc<RwLock<wde_wgpu::RenderInstanceData<'static>>>);
/// Resource storing the device limits. This is useful for pipelines to know the limits of the device and adjust their behavior accordingly.
/// It is available in both the main app and the render app, as some pipelines may need to know the limits during extraction.
#[derive(Resource, Default)]
pub struct DeviceLimits(pub Limits);
/// Resource storing the current swap chain frame. This is used to store the current frame that is being rendered to, so that it can be accessed by the render graph and the present system.
/// It is wrapped in an Option because there may not be a frame available at all times (e.g. when the window is minimized). It is also wrapped in a resource to avoid borrowing issues when accessing it from different systems.
#[derive(Resource, Default)]
pub struct SwapchainFrame {
    pub data: Option<RenderTexture>
}

/// The plugin that is responsible for the renderer.
pub(crate) struct RenderCorePlugin {
    pub window_title: String,
    pub window_resolution: (u32, u32),
    pub window_icon: Option<WindowIcon>
}
impl Plugin for RenderCorePlugin {
    fn build(&self, app: &mut App) {
        // === MAIN APP ===
        // Add window
        app.add_plugins(WindowPlugins {
            title: self.window_title.clone(),
            resolution: self.window_resolution
        })
        .insert_resource(PrimaryWindowIcon(self.window_icon.clone()))
        .init_resource::<FullscreenKey>()
        .add_systems(Update, apply_window_icon)
        .configure_sets(
            PreUpdate,
            WindowShortcutSet.after(bevy::input::InputSystems)
        )
        .add_systems(
            PreUpdate,
            toggle_fullscreen
                .in_set(WindowShortcutSet)
                .run_if(resource_exists::<Messages<bevy::input::keyboard::KeyboardInput>>)
        )
        .add_message::<SurfaceResized>()
        .add_systems(Update, send_surface_resized);

        // === RENDER APP ===
        app.add_plugins(ExtractPlugin::<RenderApp>::new(
            |_, _| {},
            Render::base,
            Render.intern(),
            RenderSet::ExtractAuto.intern(),
            RenderSet::Cleanup.intern()
        ));

        // Create the wgpu instance
        let instance = futures_lite::future::block_on(async {
            let mut system_state: SystemState<Query<&RawHandleWrapperHolder, With<PrimaryWindow>>> =
                SystemState::new(app.world_mut());
            let primary_window = system_state
                .get(app.world())
                .ok()
                .and_then(|query| query.single().ok().cloned());
            create_instance("wde_renderer", primary_window.as_ref()).await
        });
        let gpu_limits = instance.device.limits();
        let asset_server = app.world().resource::<AssetServer>().clone();
        app.insert_resource(DeviceLimits(gpu_limits.clone()));

        let render_app = app.sub_app_mut(RenderApp);
        render_app.update_schedule = Some(Render.intern());
        render_app
            .insert_resource(RenderInstance(Arc::new(RwLock::new(instance))))
            .insert_resource(asset_server)
            .insert_resource(DeviceLimits(gpu_limits))
            // Render graph
            .init_resource::<RenderGraph>()
            .add_systems(Render, RenderGraph::render.in_set(RenderSet::Render))
            // Surface
            .add_systems(
                ExtractSchedule,
                (init_surface.run_if(run_once), extract_surface_size).chain()
            )
            .add_systems(Render, prepare.in_set(RenderSet::Prepare))
            .add_systems(Render, present.in_set(RenderSet::Submit))
            .add_plugins(PipelineManagerPlugin);

        // Add the render pipeline plugins
        app.add_plugins(PipelinedRenderingPlugin);
    }
}
