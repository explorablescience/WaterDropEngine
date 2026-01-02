//! WaterDropEngine's `wde-gizmos` crate renders lightweight debug gizmos (lines/boxes) on top of
//! the main scene. It adds a dedicated render pass, a simple line-list pipeline, and a minimal
//! material model for per-gizmo color.
//!
//! # Architecture
//! - **Assets**: [`assets::cube_gizmo::CubeGizmoMesh`] builds a line-list cube mesh;
//!   [`assets::gizmo_material::GizmoMaterialAsset`] defines a single-color gizmo material with
//!   a tiny uniform buffer.
//! - **SSBO**: [`passes::gizmo_ssbo::GizmoSsbo`] allocates CPU/GPU buffers for per-instance
//!   `TransformUniform` data and exposes a storage bind group (set 1).
//! - **Pipeline**: [`passes::gizmo_pipeline::GizmoRenderPipelineAsset`] compiles the WGSL gizmo
//!   shaders (`gizmo/vert.wgsl`, `gizmo/frag.wgsl`), wiring camera (set 0), transforms (set 1),
//!   and material (set 2) layouts into a cached render pipeline.
//! - **Render pass**: [`passes::gizmo_renderpass::GizmoRenderPass`] batches gizmos by
//!   (mesh, material), uploads transforms, and draws after the main scene (pass index 1000).
//! - **Plugin wiring**: [`GizmosPlugin`] registers materials, SSBO, pipeline assets, and the
//!   render pass so gizmos always render last.
//!
//! # Quickstart (draw a debug box)
//! ```rust,no_run
//! use bevy::prelude::*;
//! use wde_renderer::prelude::*;
//! use wde_camera::prelude::*;
//! use wde_gizmos::prelude::*;
//!
//! fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         .add_plugins(RenderPlugin)
//!         .add_plugins(CameraPlugin)
//!         .add_plugins(GizmosPlugin)
//!         .add_systems(Startup, spawn_gizmo)
//!         .run();
//! }
//!
//! fn spawn_gizmo(mut commands: Commands, assets: Res<AssetServer>) {
//!     // Create material and mesh
//!     let mat: Handle<GizmoMaterialAsset> = assets.add(GizmoMaterialAsset {
//!         color: [1.0, 0.2, 0.2, 1.0],
//!         ..Default::default()
//!     });
//!     let mesh = assets.add(CubeGizmoMesh::from("gizmo-cube", Vec3::splat(1.0)));
//!
//!     commands.spawn((
//!         Mesh(mesh),
//!         GizmoMaterial(mat),
//!         Transform::from_xyz(0.0, 1.0, 0.0),
//!     ));
//! }
//! ```
//!
//! # Core usage patterns
//! - Reuse `CubeGizmoMesh` for AABBs; author custom line meshes by supplying `MeshAsset` with
//!   `RenderTopology::LineList` indices.
////! - Materials are per-entity; a single color uniform feeds all instances in the batch.
//! - Gizmos rely on the camera bind group from `wde-camera`; keep an active camera present.
//! - SSBO capacity defaults to 100k instances; adjust in code if you stream more gizmos.
//!
//! # Modules
//! - [`assets`]: gizmo mesh helper, color-only material asset + uniform packing.
//! - [`passes`]: SSBO creation, pipeline compilation, batching, and render pass submission.
//!
//! # Examples and further reading
//! - To visualize bounds, spawn multiple meshes sharing the same `GizmoMaterialAsset` so they
//!   batch together.
//! - Extend the WGSL in `res/gizmo` to add per-vertex coloring while keeping the same bind
//!   group layout.
use bevy::prelude::*;

use crate::{assets::GizmoMaterialPlugin, passes::GizmoFeaturesPlugin};

pub mod prelude {
    pub use crate::GizmosPlugin;
    pub use crate::assets::{cube_gizmo::CubeGizmoMesh, gizmo_material::{GizmoMaterial, GizmoMaterialAsset}};
}

pub mod assets;
pub mod passes;

pub struct GizmosPlugin;
impl Plugin for GizmosPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(GizmoMaterialPlugin)
            .add_plugins(GizmoFeaturesPlugin);
    }
}
