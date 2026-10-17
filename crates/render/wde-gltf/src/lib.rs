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
use std::{collections::HashMap, sync::Arc};

use serde::{Deserialize, Serialize};
use wde_logger::prelude::*;

use bevy::{
    asset::{AssetLoader, LoadContext, io::Reader},
    prelude::*
};
use wde_animation::prelude::*;
use wde_pbr::prelude::*;
use wde_renderer::prelude::*;

#[doc(hidden)]
pub mod prelude {
    pub use crate::GltfAsset;
    pub use crate::GltfError;
    pub use crate::GltfLight;
    pub use crate::GltfLightKind;
    pub use crate::GltfLoader;
    pub use crate::GltfLoaderSettings;
    pub use crate::GltfSpawnQueue;
    pub use crate::MeshGeometry;
    pub use crate::ModelCondition;
}

mod accessor;
mod error;
mod loader;
mod material;
mod model;
mod parser;

pub use error::GltfError;
pub use model::{GltfBone, GltfLight, GltfLightKind, ModelCondition};

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
    pending: Vec<(Entity, Handle<GltfAsset>)>
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
    /// Connection condition of each entry of [`Self::models`] (`None` = always shown).
    pub conditions: Vec<Option<ModelCondition>>,
    /// Triangles of each entry of [`Self::models`], kept on the CPU (e.g. for navigation): the
    /// meshes themselves leave the main world once uploaded to the GPU.
    pub geometry: Vec<Arc<MeshGeometry>>,
    /// The englobing bounding box of the entire model, computed from the bounding boxes of all meshes.
    pub bbox: MeshBbox,
    /// Named properties (e.g. spawn points) extracted from meshes whose name starts with `%P%`.
    /// These meshes are not added to [`Self::models`]; only their name, position and scale are kept here.
    pub properties: HashMap<String, (Vec3, Vec3)>,
    /// Bones of a rigged model (nodes named `%B%<bone>`), parents first.
    pub bones: Vec<GltfBone>,
    /// Bone each entry of [`Self::models`] moves with (its vertices are then relative to the bone).
    pub model_bones: Vec<Option<usize>>,
    /// Animation clips of the bones, by name.
    pub animations: HashMap<String, Handle<AnimationClip>>,
    /// Lights of the model, spawned with it (see [`Self::spawn_lights`]).
    pub lights: Vec<GltfLight>
}

impl GltfAsset {
    pub fn animation(&self, name: &str) -> Option<&Handle<AnimationClip>> {
        self.animations.get(name)
    }

    /// Spawns the lights of the model under `parent`, following its transform.
    pub fn spawn_lights(&self, commands: &mut Commands, parent: Entity) {
        for light in &self.lights {
            let color = wde_renderer::prelude::Color::LinearRgba(
                light.color.x,
                light.color.y,
                light.color.z,
                1.0
            );
            let mut entity = commands.spawn((
                Name::new(format!("Light {}", light.name)),
                light.transform,
                ChildOf(parent)
            ));
            match light.kind {
                GltfLightKind::Point => entity.insert(PointLight {
                    position: Vec3::ZERO,
                    color,
                    intensity: light.intensity,
                    range: light.range
                }),
                GltfLightKind::Spot {
                    inner_cone,
                    outer_cone
                } => entity.insert(SpotLight {
                    position: Vec3::ZERO,
                    direction: Vec3::NEG_Z,
                    color,
                    intensity: light.intensity,
                    range: light.range,
                    inner_cone,
                    outer_cone
                })
            };
        }
    }

    /// Models whose connection condition holds, `connected` telling whether an anchor is connected.
    pub fn models_where(
        &self,
        connected: impl Fn(&str) -> bool
    ) -> impl Iterator<Item = &(Handle<Mesh>, Handle<PbrMaterial>)> {
        self.models
            .iter()
            .zip(self.conditions.iter())
            .filter(move |(_, condition): &(_, &Option<ModelCondition>)| {
                condition
                    .as_ref()
                    .is_none_or(|c| connected(&c.anchor) == c.connected)
            })
            .map(|(model, _)| model)
    }
}

/// Triangles of a mesh, in model space (or bone space for meshes moving with a bone).
#[derive(Debug, Default)]
pub struct MeshGeometry {
    pub positions: Vec<Vec3>,
    pub indices: Vec<u32>
}

/// Options for loading a glTF model.
#[derive(Serialize, Deserialize, Default)]
pub struct GltfLoaderSettings {
    /// The optional stencil value to set the stencil buffer to when rendering the model. By default, no stencil value is set.
    pub stencil_value: Option<u32>
}

#[derive(Default, TypePath)]
pub(crate) struct GltfAssetLoader;
impl AssetLoader for GltfAssetLoader {
    type Asset = GltfAsset;
    type Settings = GltfLoaderSettings;
    type Error = GltfError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>
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

        // Model space transform of each bone in its rest pose
        let mut bone_matrices: Vec<Mat4> = Vec::with_capacity(model.bones.len());
        for bone in &model.bones {
            let local = Mat4::from_scale_rotation_translation(
                bone.rest.scale,
                bone.rest.rotation,
                bone.rest.translation
            );
            bone_matrices.push(match bone.parent {
                Some(parent) => bone_matrices[parent] * local,
                None => local
            });
        }

        // Add meshes to the asset server
        let mut models = Vec::new();
        let mut conditions = Vec::new();
        let mut geometry = Vec::new();
        let mut model_bones = Vec::new();
        let mut bbox_min = Vec3::splat(f32::INFINITY);
        let mut bbox_max = Vec3::splat(f32::NEG_INFINITY);
        for (i, (indices_data, vertices, material_id, condition, bone)) in
            raw_meshes.iter().enumerate()
        {
            let label = format!("gltf_mesh_{}", i);
            let (bb_min, bb_max) = bounding_boxes[i];
            let mesh_asset = Mesh {
                label: label.clone(),
                vertices: vertices.clone(),
                indices: indices_data.clone(),
                bbox: MeshBbox {
                    min: bb_min,
                    max: bb_max
                },
                use_ssbo: true
            };

            models.push((
                load_context.add_labeled_asset(label.clone(), mesh_asset),
                materials_handles[*material_id].clone()
            ));
            conditions.push(condition.clone());
            geometry.push(Arc::new(MeshGeometry {
                positions: vertices.iter().map(|v| Vec3::from(v.position)).collect(),
                indices: match indices_data.is_empty() {
                    true => (0..vertices.len() as u32).collect(),
                    false => indices_data.clone()
                }
            }));
            model_bones.push(*bone);

            // Update the overall bounding box of the model, with bone meshes in their rest pose
            let corners = match bone {
                Some(bone) => (0..8)
                    .map(|c| {
                        let corner = Vec3::select(
                            bevy::math::BVec3::new(c & 1 != 0, c & 2 != 0, c & 4 != 0),
                            bb_max,
                            bb_min
                        );
                        bone_matrices[*bone].transform_point3(corner)
                    })
                    .collect(),
                None => vec![bb_min, bb_max]
            };
            for corner in corners {
                bbox_min = bbox_min.min(corner);
                bbox_max = bbox_max.max(corner);
            }
        }

        let animations = model
            .animations
            .into_iter()
            .map(|clip| {
                let name = clip.name.clone();
                let handle = load_context.add_labeled_asset(format!("animation_{name}"), clip);
                (name, handle)
            })
            .collect();

        Ok(GltfAsset {
            path: path.to_string(),
            models,
            conditions,
            geometry,
            bbox: MeshBbox {
                min: bbox_min,
                max: bbox_max
            },
            properties: model.properties,
            bones: model.bones,
            model_bones,
            animations,
            lights: model.lights
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
    pub fn spawn(queue: &mut GltfSpawnQueue, gltf_asset: Handle<GltfAsset>, parent: Entity) {
        queue.pending.push((parent, gltf_asset));
    }

    /// Try to spawn the loaded glTF model into the Bevy world.
    /// Returns `None` if the asset handle is not loaded yet.
    pub fn try_spawn(
        commands: &mut Commands,
        gltf_asset: &Handle<GltfAsset>,
        gltf_assets: &Assets<GltfAsset>,
        parent: Entity
    ) -> Option<()> {
        let gltf_asset = gltf_assets.get(gltf_asset)?;

        // Bones, animated by the `AnimationPlayer` of an ancestor
        let mut bones: Vec<Entity> = Vec::with_capacity(gltf_asset.bones.len());
        for bone in &gltf_asset.bones {
            let bone_entity = commands
                .spawn((
                    Name::new(format!("Bone {}", bone.name)),
                    bone.rest,
                    AnimationBone {
                        name: bone.name.clone(),
                        rest: bone.rest
                    },
                    ChildOf(bone.parent.map_or(parent, |p| bones[p]))
                ))
                .id();
            bones.push(bone_entity);
        }

        for (i, (mesh_handle, material_handle)) in gltf_asset.models.iter().enumerate() {
            let mesh_parent = gltf_asset.model_bones[i].map_or(parent, |bone| bones[bone]);
            commands.spawn((
                Name::new(format!(
                    "Mesh Entity {} for GLTF Model {}",
                    i, gltf_asset.path
                )),
                Transform::default(),
                Mesh3d(mesh_handle.clone()),
                PbrMaterial3d(material_handle.clone()),
                ChildOf(mesh_parent)
            ));
        }
        gltf_asset.spawn_lights(commands, parent);
        Some(())
    }
}

fn process_gltf_spawn_queue(
    mut commands: Commands,
    mut queue: ResMut<GltfSpawnQueue>,
    gltf_assets: Res<Assets<GltfAsset>>
) {
    let mut i = 0;
    while i < queue.pending.len() {
        let (parent, gltf_asset) = &queue.pending[i];
        if GltfLoader::try_spawn(&mut commands, gltf_asset, &gltf_assets, *parent).is_some() {
            queue.pending.swap_remove(i);
        } else {
            i += 1;
        }
    }
}
