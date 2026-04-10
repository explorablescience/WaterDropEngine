//! The renderer module is responsible for rendering the scene.
//!
//! It extracts the main world into the render world and runs the render schedule.
//! It provides the [`Render`] and the [`Extract`] schedule, of the [`RenderApp`] (see [`RenderSet`] for the sets of the render schedule).
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
//! As the render app runs in a separate thread from the main app, it cannot access the main world directly. To extract data from the main world, you can use the extract schedule and the [`ExtractWorld`] system parameter. For example, to extract the camera data from the main world, you can add a system to the extract schedule:
//! ```
//! app.get_sub_app_mut(RenderApp).unwrap()
//!     .add_systems(Extract, extract_camera_data);
//!
//! fn extract_camera_data(mut commands: Commands, camera_query: ExtractWorld<Query<&Camera>>) {
//!     let camera = camera_query.single();
//!     commands.insert_resource(ExtractedCameraData {
//!         // ...
//!     });
//! }
//! ```
//!
//! ## Extracting while mutating the main world
//! If you need to extract data from the main world while also mutating it, you can use the [`MainWorld`] resource:
//! ```
//! pub fn extract_messages(
//!   mut render_test_resource: ResMut<TestResource>,
//!   mut main_world: ResMut<MainWorld>
//! ) {
//!    /* (...) */
//! }
//! ```
//!
//! ## Extracting plugins
//! If you have a plugin that has resources that need to be extracted, you can implement the [`ExtractResource`] trait for those resources and add the corresponding [`ExtractResourcePlugin`] to your plugin. For example, if you have a plugin with a resource `MyResource`, you can do:
//! ```
//! app
//!     .insert_resource(MyResource { /* ... */ })
//!     .add_plugins(ExtractResourcePlugin::<MyResource>::default());
//! ```
//! This will automatically add a system to extract `MyResource` from the main world to the render world, as long as you implement the `ExtractResource` trait for `MyResource`.
//!
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

mod extract;
mod extract_macros;
mod extract_plugin_resource;
mod render_manager;
mod render_multithread;
mod window;

pub use extract_macros::ExtractWorld;
pub use extract_plugin_resource::*;
pub use window::SurfaceResized;

use bevy::{
    app::AppLabel,
    ecs::{
        schedule::{ScheduleBuildSettings, ScheduleLabel},
        system::SystemState
    },
    prelude::*,
    tasks::futures_lite,
    window::{PrimaryWindow, RawHandleWrapperHolder}
};
use extract::{apply_extract_commands, main_extract};
use render_manager::{init_main_world, init_surface, prepare, present};
use render_multithread::PipelinedRenderingPlugin;
use std::{
    ops::{Deref, DerefMut},
    sync::{Arc, RwLock}
};
use wde_wgpu::instance::{Limits, RenderTexture, create_instance};
use window::{WindowPlugins, extract_surface_size, send_surface_resized};

use crate::passes::{PipelineManagerPlugin, RenderGraph};

/// Stores the main world for rendering as a resource.
/// This resource is only available during the extract schedule and is used to move data from the main world to the render world.
/// It is wrapped in a resource to avoid borrowing issues when extracting data from the main world.
#[derive(Resource, Default)]
pub struct MainWorld(World);
impl Deref for MainWorld {
    type Target = World;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for MainWorld {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Used to avoid allocating new worlds every frame when swapping out worlds.
#[derive(Resource, Default)]
struct EmptyWorld(World);

/// The schedule that is used to extract the main world into the render world.
/// Configure it such that it skips applying commands during the extract schedule.
/// The extract schedule will be executed when sync is called between the main app and the sub app.
#[derive(ScheduleLabel, Hash, PartialEq, Eq, Clone, Copy, Debug)]
pub struct Extract;

/// The renderer schedule set.
/// The render schedule will be executed by the renderer app.
#[derive(SystemSet, Hash, PartialEq, Eq, Clone, Copy, Debug)]
pub enum RenderSet {
    /// Run the extract commands registered during the extract schedule. This set is executed automatically and should not be used directly. Instead, use the Extract schedule.
    ExtractAuto,
    /// Prepare resources before rendering. This includes updating buffers, textures, assets, bind groups, etc.
    Prepare,
    /// Render commands.
    Render,
    /// Submit commands.
    Submit
}

/// The renderer schedule.
/// This schedule is responsible for rendering the scene.
#[derive(ScheduleLabel, Hash, PartialEq, Eq, Clone, Copy, Debug)]
pub struct Render;
impl Render {
    pub fn base() -> Schedule {
        use RenderSet::*;

        let mut schedule = Schedule::new(Self);
        schedule.configure_sets(
            (
                ExtractAuto,
                Prepare,
                Render,
                Submit
            )
            .chain()
        );

        schedule
    }
}

/// The render app. This is the app that is responsible for rendering the scene. It runs in a separate thread from the main app and has its own schedule and resources. It is used to extract the main world into the render world and run the render schedule. It is also used to manage the swap chain and present the rendered frame to the window.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, AppLabel)]
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
pub(crate) struct RenderCorePlugin;
impl Plugin for RenderCorePlugin {
    fn build(&self, app: &mut App) {
        // === MAIN APP ===
        // Add window
        app.add_plugins(WindowPlugins)
            .add_message::<SurfaceResized>()
            .add_systems(Update, send_surface_resized);

        // Add empty world component
        app.add_systems(Startup, init_main_world);

        // === RENDER APP ===
        let mut render_app = SubApp::new();
        let mut gpu_limits = None;
        {
            // Create the wgpu instance
            render_app.insert_resource(futures_lite::future::block_on(async {
                let mut system_state: SystemState<
                    Query<&RawHandleWrapperHolder, With<PrimaryWindow>>
                > = SystemState::new(app.world_mut());
                let primary_window = system_state.get(app.world()).single().ok().cloned();

                // Create the instance
                let instance = create_instance("wde_renderer", primary_window.as_ref()).await;

                // Get the GPU limits
                gpu_limits = Some(instance.device.limits());

                // Wrap the instance in an Arc<RwLock<>>
                RenderInstance(Arc::new(RwLock::new(instance)))
            }));

            // Copy the asset server from the main app
            render_app.insert_resource(app.world().resource::<AssetServer>().clone());

            // Register the extract schedule
            let mut extract_schedule = Schedule::new(Extract);
            extract_schedule.set_build_settings(ScheduleBuildSettings {
                auto_insert_apply_deferred: false,
                ..Default::default()
            });
            extract_schedule.set_apply_final_deferred(false);
            render_app.add_schedule(extract_schedule);

            // Register the render schedule that executed in parallel with the extract schedule.
            render_app.update_schedule = Some(Render.intern());
            render_app.add_schedule(Render::base());

            // Add extract command systems
            render_app
                .add_systems(
                    Render,
                    apply_extract_commands.in_set(RenderSet::ExtractAuto)
                ) // Apply the extract commands
                .set_extract(main_extract); // Register the extract commands

            // Add render graph system
            render_app
                .init_resource::<RenderGraph>()
                .add_systems(Render, RenderGraph::render.in_set(RenderSet::Render));

            // Init wgpu instance
            render_app.add_systems(
                Extract,
                (init_surface.run_if(run_once), extract_surface_size).chain()
            );

            // Add present system
            render_app
                .add_systems(Render, prepare.in_set(RenderSet::Prepare))
                .add_systems(Render, present.in_set(RenderSet::Submit));

            // Add render plugins
            render_app.add_plugins(PipelineManagerPlugin);
        }

        // Register the render app
        app.insert_sub_app(RenderApp, render_app);

        // Add the GPU limits
        app.insert_resource(DeviceLimits(gpu_limits.as_ref().unwrap().clone()));
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .insert_resource(DeviceLimits(gpu_limits.unwrap()));

        // Add the render pipeline plugins
        app.add_plugins(PipelinedRenderingPlugin);
    }
}
