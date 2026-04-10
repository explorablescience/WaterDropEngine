use wde_logger::prelude::*;

use bevy::{
    asset::{AssetLoader, LoadContext, io::Reader},
    ecs::system::{
        SystemParamItem,
        lifetimeless::{SRes, SResMut}
    },
    prelude::*
};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{BufReader, Error}
};
use thiserror::Error;
use tobj::LoadError;
use wde_wgpu::{
    buffer::{Buffer, BufferUsage},
    vertex::Vertex
};

use crate::{
    assets::{GpuBuffer, RenderAssets, SBindingOld},
    core::RenderInstance,
    utils::{SsboMesh, ssbo_mesh::SsboMeshDescriptor}
};

use super::asset::{PrepareAssetError, RenderAsset};

/// Utils component that stores a [`Mesh`] asset handle for 3D rendering.
#[derive(Component, Reflect, Default)]
pub struct Mesh3d(pub Handle<Mesh>);

#[derive(Clone, Debug)]
pub struct MeshBbox {
    pub min: Vec3,
    pub max: Vec3
}
/// Stores a CPU mesh with vertex and index data, along with metadata for GPU allocation.
/// If loaded from a file, the mesh should have a `.obj` or `.fbx` extension.
/// The `vertices`, `indices`, and `bbox` fields are expected to be populated by the asset loader; if not loaded from a file, they will default to empty and a degenerate bounding box respectively.
/// This mesh will be uploaded to the GPU, represented by a [`GpuMesh`] asset.
#[derive(Asset, TypePath, Clone, Debug)]
pub struct Mesh {
    pub label: String,

    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub bbox: MeshBbox,

    /// If true, the vertices and indices will automatically be placed in the SSBO mesh buffers [`SsboMesh`] for GPU access; if false, separate vertex and index buffers will be created with `VERTEX` and `INDEX` usage respectively. Defaults to true.
    pub use_ssbo: bool
}

/// Represents a GPU mesh resource allocated from a CPU [`Mesh`] asset.
/// If the mesh was prepared with `use_ssbo = true`, the vertex and index data will be in the SSBO mesh buffer [`SsboMesh`] and the `vertex_buffer` and `index_buffer` fields will be `None`. Otherwhise, they will contain GPU buffers with the vertex and index data respectively.
pub struct GpuMesh {
    pub label: String,

    /// Offset to the vertex buffer in the [`SsboMesh`] if `use_ssbo` is true. 0 otherwise.
    pub ssbo_first_vertex: u32,
    /// Offset to the index buffer in the [`SsboMesh`] if `use_ssbo` is true. 0 otherwise.
    pub ssbo_first_index: u32,
    pub use_ssbo: bool,

    /// If not using SSBO, the GPU vertex buffer containing `Vertex` data. `None` if `use_ssbo` is true.
    pub vertex_buffer: Option<Buffer>,
    /// If not using SSBO, the GPU index buffer containing `u32` index data. `None` if `use_ssbo` is true.
    pub index_buffer: Option<Buffer>,

    pub index_count: u32,
    pub bounding_box: MeshBbox
}
impl RenderAsset for GpuMesh {
    type SourceAsset = Mesh;
    type Params = (
        SRes<RenderInstance>,
        SResMut<SsboMeshDescriptor>,
        SBindingOld<SsboMesh>,
        SRes<RenderAssets<GpuBuffer>>
    );

    fn prepare(
        asset: Self::SourceAsset,
        (render_instance, ssbo_descriptor, ssbo_mesh, gpu_buffers): &mut SystemParamItem<
            Self::Params
        >
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        trace!(asset.label, "Preparing GPU mesh asset.");

        // Get the SSBO mesh resource
        let ssbo = match ssbo_mesh.iter().next() {
            Some((_, ssbo)) => ssbo,
            None => return Err(PrepareAssetError::RetryNextUpdate(asset))
        };

        // Get the ssbo buffers
        let (ssbo_vertex_buffer, ssbo_index_buffer) = match (
            gpu_buffers.get(ssbo.get_buffer(SsboMesh::VERTEX_BUFFER_ID).unwrap()),
            gpu_buffers.get(ssbo.get_buffer(SsboMesh::INDEX_BUFFER_ID).unwrap())
        ) {
            (Some(vb), Some(ib)) => (vb, ib),
            _ => return Err(PrepareAssetError::RetryNextUpdate(asset))
        };

        // Buffer usage
        let usage_vertex = if asset.use_ssbo {
            BufferUsage::COPY_SRC
        } else {
            BufferUsage::VERTEX
        };
        let usage_index = if asset.use_ssbo {
            BufferUsage::COPY_SRC
        } else {
            BufferUsage::INDEX
        };

        // Create staging buffers
        let render_instance = render_instance.0.read().unwrap();
        let vertex_buffer = Buffer::new(
            &render_instance,
            format!("{}-vertex-staging", asset.label).as_str(),
            std::mem::size_of::<Vertex>() * asset.vertices.len(),
            usage_vertex,
            Some(bytemuck::cast_slice(&asset.vertices))
        );
        let index_buffer = Buffer::new(
            &render_instance,
            format!("{}-indices-staging", asset.label).as_str(),
            std::mem::size_of::<u32>() * asset.indices.len(),
            usage_index,
            Some(bytemuck::cast_slice(&asset.indices))
        );

        // If not using SSBO, return the buffers directly
        if !asset.use_ssbo {
            return Ok(GpuMesh {
                label: asset.label,
                ssbo_first_vertex: 0,
                ssbo_first_index: 0,
                index_count: asset.indices.len() as u32,
                bounding_box: asset.bbox,
                use_ssbo: asset.use_ssbo,
                vertex_buffer: Some(vertex_buffer),
                index_buffer: Some(index_buffer)
            });
        }

        // Copy to GPU buffers
        let first_vertex = ssbo_descriptor.vertex_buffer_offset;
        let first_index = ssbo_descriptor.index_buffer_offset;
        let vertices_count = asset.vertices.len() as u32;
        let indices_count = asset.indices.len() as u32;

        // Calculate byte offsets and sizes for buffer copy operations
        let vertices_offset_bytes = (first_vertex as u64) * (std::mem::size_of::<Vertex>() as u64);
        let indices_offset_bytes = (first_index as u64) * (std::mem::size_of::<u32>() as u64);
        let vertices_size_bytes = (vertices_count as u64) * (std::mem::size_of::<Vertex>() as u64);
        let indices_size_bytes = (indices_count as u64) * (std::mem::size_of::<u32>() as u64);

        ssbo_vertex_buffer.buffer.copy_from_buffer_offset(
            &render_instance,
            &vertex_buffer,
            0,
            vertices_offset_bytes,
            vertices_size_bytes
        );
        ssbo_descriptor.vertex_buffer_offset += vertices_count;

        ssbo_index_buffer.buffer.copy_from_buffer_offset(
            &render_instance,
            &index_buffer,
            0,
            indices_offset_bytes,
            indices_size_bytes
        );
        ssbo_descriptor.index_buffer_offset += indices_count;

        Ok(GpuMesh {
            label: asset.label,
            ssbo_first_vertex: first_vertex,
            ssbo_first_index: first_index,
            index_count: indices_count,
            bounding_box: asset.bbox,
            use_ssbo: asset.use_ssbo,
            vertex_buffer: None,
            index_buffer: None
        })
    }

    fn label(&self) -> &str {
        &self.label
    }
}

/// Settings for loading a mesh asset while creating a [`Mesh`].
#[derive(Serialize, Deserialize)]
pub struct MeshLoaderSettings {
    pub label: String,
    /// Should the vertices and indices be in the SSBO mesh buffers? Defaults to true. If false, separate vertex and index buffers will be created.
    pub use_ssbo: bool
}
impl Default for MeshLoaderSettings {
    fn default() -> Self {
        Self {
            label: "Unknown Mesh".to_string(),
            use_ssbo: true
        }
    }
}

#[derive(Debug, Error)]
pub(crate) enum MeshLoaderError {
    #[error("Could not load mesh: {0}")]
    Io(#[from] std::io::Error)
}
#[derive(Default, TypePath)]
pub(crate) struct MeshLoader;
impl AssetLoader for MeshLoader {
    type Asset = Mesh;
    type Settings = MeshLoaderSettings;
    type Error = MeshLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        settings: &Self::Settings,
        load_context: &mut LoadContext<'_>
    ) -> Result<Self::Asset, Self::Error> {
        info!("Loading mesh {}.", load_context.path());

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
            Err(e) => return Err(MeshLoaderError::Io(Error::other(e.to_string())))
        };
        let models = load_res.0;

        // Load models
        let mut vertices: Vec<Vertex> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();
        let mut bounding_box = MeshBbox {
            min: Vec3::new(f32::MAX, f32::MAX, f32::MAX),
            max: Vec3::new(f32::MIN, f32::MIN, f32::MIN)
        };
        for m in models.iter() {
            let mesh = &m.mesh;
            if mesh.positions.len() % 3 != 0 {
                return Err(MeshLoaderError::Io(std::io::Error::other(
                    "Mesh positions are not divisible by 3."
                )));
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
                    tangent: [0.0, 0.0, 0.0, 0.0]
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
        Ok(Mesh {
            label,
            vertices,
            indices,
            bbox: bounding_box,
            use_ssbo: settings.use_ssbo
        })
    }

    fn extensions(&self) -> &[&str] {
        &["obj", "fbx"]
    }
}
