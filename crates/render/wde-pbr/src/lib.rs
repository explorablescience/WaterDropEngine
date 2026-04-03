//! WaterDropEngine's `wde-pbr` crate implements a deferred PBR pipeline on top of `wde-renderer`.
//! It introduces PBR materials, light components, deferred G-buffer targets, and two render passes
//! (G-buffer + lighting).
//!
//! # Architecture
//! - **Assets**: [`assets::PbrMaterialAsset`] stores albedo/specular colors and optional textures;
//!   the material builder emits a uniform (flags, albedo, specular) plus optional sampled textures.
//! - **Components**: [`components::lights`] defines [`components::lights::DirectionalLight`], [`components::lights::PointLight`],
//!   [`components::lights::SpotLight`] with attenuation helpers and defaults.
//! - **Features**: `features::lights::LightsFeatureBuffer` extracts all lights each frame into a
//!   storage buffer + bind group (set 3 in lighting), keeping count in element 0.
//! - **SSBO**: `passes::pbr_ssbo::PbrSsbo` mirrors instance transforms for meshes rendered in
//!   the G-buffer pass (storage at set 1).
//! - **Deferred targets**: `passes::pbr_textures::PbrDeferredTextures` manages albedo/normal/
//!   material render targets and rebuilds them on resize, with a bind group (set 2 in lighting).
//! - **Pipelines**:
//!   - `passes::pbr_pipeline_gbuffer::PbrGBufferRenderPipelineAsset` compiles
//!     `pbr/gbuffer_vert.wgsl` + `pbr/gbuffer_frag.wgsl`, outputs into the three deferred textures,
//!     and consumes camera (set 0), instance SSBO (set 1), and PBR material (set 2).
//!   - `passes::pbr_pipeline_lighting::PbrLightingRenderPipelineAsset` compiles
//!     `pbr/lighting_vert.wgsl` + `pbr/lighting_frag.wgsl`, sampling depth, deferred textures, and
//!     lights to produce the final frame.
//! - **Render passes**:
//!   - `passes::pbr_renderpass_gbuffer::PbrGBufferRenderPass` batches meshes by
//!     (mesh, material), uploads transforms to the SSBO, and writes G-buffer attachments.
//!   - `passes::pbr_renderpass_lighting::PbrLightingRenderPass` draws a fullscreen quad and
//!     applies lighting using camera, depth, deferred, and lights bind groups.
//! - **Plugin wiring**: [`PbrPlugin`] registers assets/components, sets up SSBO + deferred
//!   textures, initializes both pipelines, and registers the two passes in the render graph
//!   (G-buffer before lighting).
//!
//! # Quickstart (lit cube)
//! ```rust,no_run
//! use bevy::prelude::*;
//! use wde_renderer::prelude::*;
//! use wde_camera::prelude::*;
//! use wde_pbr::prelude::*;
//!
//! fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         .add_plugins(RenderPlugin)
//!         .add_plugins(CameraPlugin)
//!         .add_plugins(PbrPlugin)
//!         .add_systems(Startup, setup)
//!         .run();
//! }
//!
//! fn setup(mut commands: Commands, assets: Res<AssetServer>) {
//!     // Camera
//!     commands.spawn((
//!         wde_camera::components::Camera,
//!         wde_camera::components::CameraView::default(),
//!         wde_camera::components::CameraController::default(),
//!         wde_camera::components::ActiveCamera,
//!         Transform::from_xyz(0.0, 1.5, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
//!     ));
//!
//!     // Light
//!    commands.spawn((
//!         components::lights::DirectionalLight { direction: Vec3::new(-1.0, -1.0, -0.5), ..Default::default() },
//!     ));
//!
//!     // Material + mesh
//!     let mat = assets.add(PbrMaterialAsset {
//!         albedo: (0.8, 0.7, 0.6, 1.0),
//!         ..Default::default()
//!     });
//!     let mesh = assets.add(wde_renderer::prelude::meshes::CubeMesh::from("pbr-cube", 1.0));
//!     commands.spawn((
//!         Mesh(mesh),
//!         crate::assets::PbrMaterial(mat),
//!         Transform::from_xyz(0.0, 0.5, 0.0),
//!     ));
//! }
//! ```
//!
//! # Core usage patterns
//! - Keep lights lean: first element of the light buffer stores count; default cap is 64.
//! - Use `PbrMaterialAsset` flags to mix textures and constants; absent textures fall back to
//!   uniform values.
//! - All PBR draws go through the G-buffer pass; lighting runs fullscreen and must see up-to-date
//!   depth + deferred bind group + lights bind group.
//! - Resize events trigger deferred texture recreation; the lighting bind group rebuilds lazily.
//!
//! # Modules
//! - [`assets`]: PBR material asset, GPU packing, and registration.
//! - [`components`]: light components and CPU→GPU storage layout.
//! - [`features`]: light extraction and bind group construction.
//! - [`passes`]: SSBOs, deferred targets, pipelines, and G-buffer/lighting render passes.
//!
//! # Examples and further reading
//! - Extend `PbrMaterialAsset` with metallic/roughness maps by adding extra bindings in
//!   `describe()` and mirroring them in WGSL.
//! - For clustered/forward variants, reuse the light buffer format and swap the pipelines.
#![allow(clippy::type_complexity)]
use bevy::prelude::*;
use wde_renderer::prelude::*;

use crate::{assets::{PbrAssetsPlugin, PbrMaterial, PbrMaterialAsset}, components::PbrComponentsPlugin, logic::PbrLogicPlugin, passes::PbrFeaturesPlugin as PbrPassesPlugin};

pub mod prelude {
    pub use crate::PbrPlugin;
    pub use crate::assets::{PbrMaterial, PbrMaterialAsset};
    pub use crate::components::{lights::*, model::*};
    pub use crate::passes::*;
}

pub mod assets;
pub mod components;
mod passes;
mod logic;

pub struct PbrPlugin;
impl Plugin for PbrPlugin {
    fn build(&self, app: &mut App) {
        // Add the different plugins
        app
            .add_plugins(PbrAssetsPlugin)
            .add_plugins(PbrComponentsPlugin)
            .add_plugins(PbrPassesPlugin)
            .add_plugins(PbrLogicPlugin);

        // Always create a dummy resource such that the render pass extraction can run
        app.add_systems(Startup, init_dummy_element);
    }
}

fn init_dummy_element(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Transform::from_xyz(1000.0, 1000.0, 1000.0).with_scale(Vec3::ZERO),
        Mesh(asset_server.add(CubeMesh::from("dummy", 1.0))),
        PbrMaterial(asset_server.add(PbrMaterialAsset {
            label: "dummy".to_string(),
            albedo: (0.0, 0.0, 0.0, 0.0),
            metallic: 0.0,
            ..Default::default()
        }))
    ));
}
