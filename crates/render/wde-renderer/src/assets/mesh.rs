//! Mesh asset types and GPU preparation pipeline.
//!
//! CPU meshes are loaded from disk via `MeshLoader` (OBJ/FBX) or constructed
//! procedurally in code, then uploaded as `GpuMesh` buffers through the render
//! assets pipeline. Bounding boxes are tracked for culling and debug helpers.

use std::{fs::File, io::BufReader};

use wde_logger::prelude::*;
use bevy::{asset::{AssetLoader, LoadContext, io::Reader}, ecs::system::lifetimeless::SRes, prelude::*};
use thiserror::Error;
use serde::{Deserialize, Serialize};
use tobj::LoadError;
use wde_wgpu::{buffer::{BufferUsage, Buffer}, vertex::Vertex};

use crate::core::RenderInstance;

use super::render_assets::{PrepareAssetError, RenderAsset};

/// The bounding box of the model.
#[derive(Clone, Debug)]
pub struct ModelBoundingBox {
    /// The minimum point of the bounding box.
    pub min: Vec3,
    /// The maximum point of the bounding box.
    pub max: Vec3,
}
impl Default for ModelBoundingBox {
    fn default() -> Self {
        Self {
            min: Vec3::new(f32::MAX, f32::MAX, f32::MAX),
            max: Vec3::new(f32::MIN, f32::MIN, f32::MIN),
        }
    }
}

#[derive(Component, Reflect, Default)]
/// Scene component referencing a CPU mesh asset handle.
pub struct Mesh(pub Handle<MeshAsset>);
impl Serialize for Mesh {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&format!("Mesh({})", self.0.id().untyped()))
    }
}

#[derive(Asset, TypePath, Clone)]
pub struct MeshAsset {
    /// Debug label for the mesh; propagated to GPU buffer labels.
    pub label: String,
    /// Vertex list in object space.
    pub vertices: Vec<Vertex>,
    /// Triangle indices referencing `vertices`.
    pub indices: Vec<u32>,
    /// Axis-aligned bounding box in object space.
    pub bounding_box: ModelBoundingBox,
}

#[derive(Default, TypePath)]
pub struct MeshLoader;

#[derive(Serialize, Deserialize)]
/// Load-time configuration for [`MeshLoader`].
pub struct MeshLoaderSettings {
    /// Label to apply to the loaded mesh; defaults to the asset path when empty.
    pub label: String,
}

impl Default for MeshLoaderSettings {
    fn default() -> Self {
        Self { label: "".to_string() }
    }
}

#[derive(Debug, Error)]
pub enum MeshLoaderError {
    #[error("Could not load mesh: {0}")]
    Io(#[from] std::io::Error),
}

impl AssetLoader for MeshLoader {
    type Asset = MeshAsset;
    type Settings = MeshLoaderSettings;
    type Error = MeshLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        info!("Loading mesh on the CPU from {}.", load_context.path());

        // Update the label from the path
        let label = if settings.label.is_empty() {
            load_context.path().to_string()
        } else {
            settings.label.clone()
        };

        // Read the texture data
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;

        // Open file
        #[allow(clippy::blocks_in_conditions)]
        let load_res = match tobj::load_obj_buf(
            &mut BufReader::new(bytes.as_slice()),
            &tobj::LoadOptions {
                single_index: true,
                ..Default::default()
            },
            |p| {
                let f = match File::open(p.file_name().unwrap().to_str().unwrap()) {
                    Ok(f) => f,
                    Err(_) => return Err(LoadError::OpenFileFailed)
                };
                tobj::load_mtl_buf(&mut BufReader::new(f))
            }
        ) {
            Ok(res) => res,
            Err(e) => return Err(MeshLoaderError::Io(std::io::Error::other(e.to_string()))),
        };
        let models = load_res.0;

        // Bounding box of the model
        let mut bounding_box = ModelBoundingBox {
            min: Vec3::new(f32::MAX, f32::MAX, f32::MAX),
            max: Vec3::new(f32::MIN, f32::MIN, f32::MIN),
        };

        // Load models
        let mut vertices: Vec<Vertex> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();
        for m in models.iter() {
            let mesh = &m.mesh;
            if mesh.positions.len() % 3 != 0 {
                return Err(MeshLoaderError::Io(std::io::Error::other("Mesh positions are not divisible by 3.")));
            }

            // Allocate sizes
            vertices.reserve(mesh.positions.len() / 3);

            // Create vertices
            for vtx in 0..mesh.positions.len() / 3 {
                let x = mesh.positions[3 * vtx];
                let y = mesh.positions[3 * vtx + 1];
                let z = mesh.positions[3 * vtx + 2];

                // Normals
                let mut nx = 0.0;
                let mut ny = 0.0;
                let mut nz = 0.0;
                if mesh.normals.len() >= 3 * vtx + 2 {
                    nx = mesh.normals[3 * vtx];
                    ny = mesh.normals[3 * vtx + 1];
                    nz = mesh.normals[3 * vtx + 2];
                }

                // UVs
                let mut u = 0.0;
                let mut v = 0.0;
                if mesh.texcoords.len() > 2 * vtx {
                    u = mesh.texcoords[2 * vtx];
                    v = mesh.texcoords[2 * vtx + 1];
                }

                // Vertex
                vertices.push(Vertex {
                    position: [x, y, z],
                    normal: [nx, ny, nz],
                    uv: [u, v],
                    tangent: [0.0, 0.0, 0.0, 0.0],
                });

                // Update bounding box
                bounding_box.min.x = bounding_box.min.x.min(x);
                bounding_box.min.y = bounding_box.min.y.min(y);
                bounding_box.min.z = bounding_box.min.z.min(z);
                bounding_box.max.x = bounding_box.max.x.max(x);
                bounding_box.max.y = bounding_box.max.y.max(y);
                bounding_box.max.z = bounding_box.max.z.max(z);
            }

            // Push indices
            indices.extend_from_slice(&mesh.indices);
        }

        // Return the mesh
        Ok(MeshAsset {
            label,
            vertices, indices, bounding_box
        })
    }

    fn extensions(&self) -> &[&str] {
        &["obj", "fbx"]
    }
}



pub struct GpuMesh {
    /// Copy of the CPU label applied to GPU buffers for debugging.
    pub label: String,
    /// GPU vertex buffer containing tightly packed [`Vertex`] data.
    pub vertex_buffer: Buffer,
    /// GPU index buffer containing `u32` indices.
    pub index_buffer: Buffer,
    /// Total index count for draw calls.
    pub index_count: u32,
    /// Axis-aligned bounding box in object space, mirrored from CPU asset.
    pub bounding_box: ModelBoundingBox,
}
impl RenderAsset for GpuMesh {
    type SourceAsset = MeshAsset;
    type Param = SRes<RenderInstance>;

    fn prepare_asset(
            asset: Self::SourceAsset,
            render_instance: &mut bevy::ecs::system::SystemParamItem<Self::Param>,
        ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        debug!(asset.label, "Loading mesh on the GPU.");

        // Create vertex buffer
        let render_instance = render_instance.0.read().unwrap();
        let vertex_buffer = Buffer::new(
            &render_instance,
            format!("{}-vertex", asset.label).as_str(),
            std::mem::size_of::<Vertex>() * asset.vertices.len(),
            BufferUsage::VERTEX,
            Some(bytemuck::cast_slice(&asset.vertices)));

        // Create index buffer
        let index_buffer = Buffer::new(
            &render_instance,
            format!("{}-indices", asset.label).as_str(),
            std::mem::size_of::<u32>() * asset.indices.len(),
            BufferUsage::INDEX,
            Some(bytemuck::cast_slice(&asset.indices)));
        
        Ok(GpuMesh {
            label: asset.label,
            vertex_buffer,
            index_buffer,
            index_count: asset.indices.len() as u32,
            bounding_box: asset.bounding_box,
        })
    }

    fn label(&self) -> &str {
        &self.label
    }
}
