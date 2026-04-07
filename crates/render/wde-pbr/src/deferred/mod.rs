//! Deferred PBR module.
//!
//! This module connects PBR data (materials, models, lights) to the engine deferred
//! render graph.
//!
//! # PBR Overview
//! The PBR rendering stack is composed of two passes.
//!
//! ## GBuffer pass (see [`crate::RenderPassDeferredGBuffer`])
//! This pass is composed of a few steps that run every frame:
//! - It extracts every entity with a [`Transform`] and [`PbrModel`] (which have a vector of [`Mesh`] and [`PbrMaterial`] elements) from the main world to the render world, and stores them in the `PbrModelRegistry` resource. Each model element is assigned a unique UUID, as well as a transform ID that corresponds to its entry in the GPU [SsboTransform] buffer.
//! - When a transform or a model is added or changed in the main world, the corresponding transform is marked as dirty, and the [SsboTransform] buffer is updated with the new transform data at the next frame.
//! - Then, for each frame, a `Batches` resource is created from the extracted entities, sorted by their material handle to minimize render pipeline changes. Each `Batch` points to the corresponding mesh and material handles, as well as an instance index. It then uses another buffer to store pointers from the entity instance indices to their transform IDs in the [SsboTransform] buffer (see `ModelUuidToTransformUuidRender`).
//! - Finally, the render pass iterates over the batches and renders them, writing to the G-buffer [`crate::DeferredTextures`] and the depth texture.
//! - It then resolves the G-buffer textures to `DeferredTexturesResolved` for use in the lighting pass, which doesn't use MSAA.
//!
//! ## Lighting pass (see [`RenderPassDeferredLighting`])
//! - This pass first extracts the light data from the main world to the render world, and stores it in the `LightsSsbo` ssbo. Supported light types are currently [DirectionalLight], [PointLight] and [SpotLight].
//! - Then, for each frame, it reads the G-buffer textures and the light data, and computes the final lit colors, which it writes to a [`RenderTexture`] in MSAA format.
//!
//! ## Limitations
//! The number of supported lights and tracked PBR transform entries is limited by:
//!   - [`MAX_LIGHTS`] limits the GPU light buffer size.
//!   - [`SSBO_TRANSFORM_MAX_ENTITY`] limits tracked PBR transform entries.
//!
//! ## Custom rendering
//! To find out more about how to create custom subpasses, materials and render pipelines, see the documentation of the [`wde_renderer`] crate.
//!
//! # Usage
//! ```rust
//! // Spawn a directional PBR light
//! commands.spawn(DirectionalLight {
//!     direction: Vec3::new(-0.5, -1.0, -0.25).normalize(),
//!     ..Default::default()
//! });
//!
//! // Spawn a simple PBR model (with a single mesh and material)
//! commands.spawn((
//!     Transform::from_xyz(0.0, 0.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
//!     PbrModel(vec![(
//!         asset_server.add("models/example_model.obj"),
//!         asset_server.add(PbrMaterial {
//!             albedo: Color::WHITE,
//!             metallic: 0.5,
//!             roughness: 0.5,
//!             ..Default::default()
//!         })
//!     )]
//! ));
//! ```
use bevy::prelude::*;
use wde_renderer::prelude::*;

mod dependencies;
mod lights;
mod subpass;

pub use dependencies::*;
pub use lights::*;

use crate::{
    deferred::{
        dependencies::DeferredDependenciesPlugin, lights::LightsPlugin, subpass::PbrRenderPlugin
    },
    passes::{RenderPassDeferredLighting, SubRenderPassLightingPbr}
};

pub(crate) struct DeferredPlugin;
impl Plugin for DeferredPlugin {
    fn build(&self, app: &mut App) {
        // Add the plugins
        app.add_plugins((LightsPlugin, DeferredDependenciesPlugin, PbrRenderPlugin));

        // Add the render graph nodes
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .world_mut()
            .get_resource_mut::<RenderGraph>()
            .unwrap()
            .add_sub_pass::<SubRenderPassLightingPbr, RenderPassDeferredLighting>();
    }
}
