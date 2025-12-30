//! The renderer module is responsible for rendering the scene.
//! It extracts the main world into the render world and runs the render schedule.

pub mod window;
pub mod extract;
pub mod render_manager;
pub mod extract_macros;
pub mod render_multithread;

use bevy::{app::AppLabel, ecs::{schedule::{ScheduleBuildSettings, ScheduleLabel}, system::SystemState}, prelude::*, tasks::futures_lite, window::{PrimaryWindow, RawHandleWrapperHolder}};
use extract::{apply_extract_commands, main_extract};
use render_manager::{init_main_world, init_surface, prepare, present};
use render_multithread::PipelinedRenderingPlugin;
use wde_wgpu::instance::{create_instance, Limits, RenderTexture};
use window::{extract_surface_size, send_surface_resized, SurfaceResized, WindowPlugins};
use std::ops::{Deref, DerefMut};

use crate::{components:: RenderComponentsPlugin, features::RenderFeaturesPlugin, passes::{render_graph::RenderGraph, RendererPlugin}, pipelines::PipelineManagerPlugin};


/// Stores the main world for rendering as a resource.
#[derive(Resource, Default)]
pub(crate) struct MainWorld(World);

impl Deref for MainWorld {
    type Target = World;
    fn deref(&self) -> &Self::Target { &self.0 }
}

impl DerefMut for MainWorld {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
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
    /// Run the extract commands registered during the extract schedule.
    ExtractCommands,
    /// Initialize the newly created assets.
    PrepareAssets,
    /// Prepare resources.
    Prepare,
    /// Prepare the bind groups.
    BindGroups,
    /// Process the logic just before rendering.
    Process,
    /// Render commands.
    Render,
    /// Submit commands.
    Submit,
    /// Cleanup resources.
    Cleanup,
}

/// The renderer schedule.
/// This schedule is responsible for rendering the scene.
#[derive(ScheduleLabel, Hash, PartialEq, Eq, Clone, Copy, Debug)]
pub struct Render;

impl Render {
    pub fn base() -> Schedule {
        use RenderSet::*;

        let mut schedule = Schedule::new(Self);
        schedule.configure_sets((
            ExtractCommands,
            PrepareAssets,
            Prepare,
            BindGroups,
            Process,
            Render,
            Submit,
            Cleanup,
        ).chain());

        schedule
    }
}

#[derive(Resource, Default)]
pub struct DeviceLimits(pub Limits);


#[derive(Resource, Default)]
pub struct SwapchainFrame {
    pub data: Option<RenderTexture>,
}



/// The main app for the renderer.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, AppLabel)]
pub struct RenderApp;


/// The plugin that is responsible for the renderer.
pub struct RenderCorePlugin;
impl Plugin for RenderCorePlugin {
    fn build(&self, app: &mut App) {
        // === MAIN APP ===
        // Add window
        app
            .add_plugins(WindowPlugins)
            .add_event::<SurfaceResized>()
            .add_systems(Update, send_surface_resized);

        // Add empty world component
        app.add_systems(Startup, init_main_world);



        // === RENDER APP ===
        let mut render_app = SubApp::new();
        let mut gpu_limits = None;
        {
            // Create the wgpu instance
            render_app.insert_resource(futures_lite::future::block_on(async {
                let mut system_state: SystemState<Query<&RawHandleWrapperHolder, With<PrimaryWindow>>> = SystemState::new(app.world_mut());
                let primary_window = system_state.get(app.world()).single().ok().cloned();
                let instance = create_instance("wde_renderer", primary_window.as_ref()).await;
                gpu_limits = Some(instance.data.read().unwrap().device.limits());
                instance
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
                .add_systems(Render, 
                    apply_extract_commands.in_set(RenderSet::ExtractCommands)) // Apply the extract commands
                .set_extract(main_extract); // Register the extract commands

            // Add render graph system
            render_app
                .init_resource::<RenderGraph>()
                .add_systems(Render, RenderGraph::render.in_set(RenderSet::Render));

            // Init wgpu instance
            render_app.add_systems(Extract, (init_surface.run_if(run_once), extract_surface_size).chain());

            // Add present system
            render_app
                .add_systems(Render, prepare.in_set(RenderSet::Prepare))
                .add_systems(Render, present.in_set(RenderSet::Submit));

            // Add render plugins
            render_app
                .add_plugins(PipelineManagerPlugin);
        }

        // Register the render app
        app.insert_sub_app(RenderApp, render_app);

        // Add the GPU limits
        app.insert_resource(DeviceLimits(gpu_limits.as_ref().unwrap().clone()));
        app.get_sub_app_mut(RenderApp).unwrap().insert_resource(DeviceLimits(gpu_limits.unwrap()));

        // Add the render pipeline plugins
        app
            .add_plugins(RendererPlugin)
            .add_plugins(PipelinedRenderingPlugin)
            .add_plugins(RenderComponentsPlugin)
            .add_plugins(RenderFeaturesPlugin);
    }
}
