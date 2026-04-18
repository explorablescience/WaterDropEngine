//! GLTF Loader for WaterDropEngine
//!
//! This crate provides functionality to load and parse glTF files, converting them into assets that can be used within the WaterDropEngine. It supports loading meshes, materials, and textures defined in glTF files and integrates them with the engine's rendering system.
//!
//! # Example
//! To load a glTF model, you can use the [`GltfLoader`](crate::GltfLoader) in your `setup` system as follows:
//! ```rust
//! let gltf_asset = asset_server.load("models/model.gltf");
//! GltfLoader::spawn(&mut gltf_spawn_queue, gltf_asset);
//! ```
//! Note that the `gltf_asset` handle will be enqueued for spawning once it is loaded. The actual spawning of the model into the world will happen automatically in the background, and you can check for its presence using the asset handle.
//! The rendering of the loaded model is then managed by the [`wde_pbr`](wde_pbr) crate, which handles the materials and shaders for the meshes.
use wde_logger::prelude::*;

use bevy::{
    asset::{AssetLoader, LoadContext, io::Reader},
    prelude::*,
};
use wde_pbr::prelude::*;
use wde_renderer::prelude::*;

#[doc(hidden)]
pub mod prelude {
    pub use crate::GltfAsset;
    pub use crate::GltfError;
    pub use crate::GltfLoader;
    pub use crate::GltfSpawnQueue;
}

mod accessor;
mod error;
mod loader;
mod material;
mod model;
mod parser;

pub use error::GltfError;

pub struct GltfPlugin;
impl Plugin for GltfPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<GltfAsset>()
            .init_asset_loader::<GltfAssetLoader>()
            .init_resource::<GltfSpawnQueue>()
            .add_systems(Update, process_gltf_spawn_queue);
    }
}

/// Queue of glTF assets to spawn once they are loaded.
#[derive(Resource, Default)]
pub struct GltfSpawnQueue {
    pending: Vec<Handle<GltfAsset>>,
}

/// Representation of a 3D GLTF model asset.
/// This will spawn the model and return the parent entity ID.
/// See the [crate] documentation for usage examples.
#[derive(Asset, TypePath, Clone)]
pub struct GltfAsset {
    path: String,

    /// The list of parsed glTF models.
    /// Each model is represented by a mesh and its associated material.
    pub models: Vec<(Handle<Mesh>, Handle<PbrMaterial>)>,
}

#[derive(Default, TypePath)]
pub(crate) struct GltfAssetLoader;
impl AssetLoader for GltfAssetLoader {
    type Asset = GltfAsset;
    type Settings = ();
    type Error = GltfError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let path = load_context.path().clone();
        debug!("Loading glTF file {}.", &path);

        // Parse the glTF file
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let model = parser::parse_gltf(bytes, &path.to_string())?;

        // Form and load the model into the Bevy world
        let (raw_materials, raw_meshes, bounding_boxes) = loader::form_models(&model)?;

        // Construct materials
        let materials_handles: Vec<Handle<PbrMaterial>> = raw_materials
            .iter()
            .map(|material| material.to_pbr(load_context))
            .collect();

        // Add meshes to the asset server
        let mut models = Vec::new();
        for (i, (indices_data, vertices, material_id)) in raw_meshes.iter().enumerate() {
            let label = format!("gltf_mesh_{}", i);
            let (bb_min, bb_max) = bounding_boxes[i];
            let mesh_asset = Mesh {
                label: label.clone(),
                vertices: vertices.clone(),
                indices: indices_data.clone(),
                bbox: MeshBbox {
                    min: bb_min,
                    max: bb_max,
                },
                use_ssbo: true,
            };

            models.push((
                load_context.add_labeled_asset(label.clone(), mesh_asset),
                materials_handles[*material_id].clone(),
            ));
        }

        Ok(GltfAsset {
            path: path.to_string(),
            models,
        })
    }

    fn extensions(&self) -> &[&str] {
        &["gltf", "glb"]
    }
}

/// Manager to load glTF models into the Bevy world.
/// See the [crate] documentation for usage examples.
pub struct GltfLoader;
impl GltfLoader {
    /// Enqueue a glTF asset handle to be spawned automatically when loaded.
    pub fn spawn(queue: &mut GltfSpawnQueue, gltf_asset: Handle<GltfAsset>) {
        queue.pending.push(gltf_asset);
    }

    /// Try to spawn the loaded glTF model into the Bevy world.
    /// Returns `None` if the asset handle is not loaded yet.
    pub fn try_spawn(
        commands: &mut Commands,
        gltf_asset: &Handle<GltfAsset>,
        gltf_assets: &Assets<GltfAsset>,
    ) -> Option<Entity> {
        let gltf_asset = gltf_assets.get(gltf_asset)?;
        let parent_entity = commands
            .spawn((
                Name::new(format!("GLTF Model {}", gltf_asset.path)),
                Transform::default(),
            ))
            .id();
        for (i, (mesh_handle, material_handle)) in gltf_asset.models.iter().enumerate() {
            commands.spawn((
                Name::new(format!(
                    "Mesh Entity {} for GLTF Model {}",
                    i, gltf_asset.path
                )),
                Transform::default(),
                Mesh3d(mesh_handle.clone()),
                PbrMaterial3d(material_handle.clone()),
                ChildOf(parent_entity),
            ));
        }

        Some(parent_entity)
    }
}

fn process_gltf_spawn_queue(
    mut commands: Commands,
    mut queue: ResMut<GltfSpawnQueue>,
    gltf_assets: Res<Assets<GltfAsset>>,
) {
    let mut i = 0;
    while i < queue.pending.len() {
        if GltfLoader::try_spawn(&mut commands, &queue.pending[i], &gltf_assets).is_some() {
            queue.pending.swap_remove(i);
        } else {
            i += 1;
        }
    }
}
