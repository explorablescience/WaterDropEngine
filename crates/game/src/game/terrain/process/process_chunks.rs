use std::hash::Hash;

use bevy::{ecs::world::CommandQueue, platform::collections::HashMap, prelude::*, tasks::{AsyncComputeTaskPool, Task, block_on, futures_lite::future}};
use wde_render::assets::Buffer;
use wde_wgpu::{buffer::BufferUsage, vertex::WVertex};

use crate::terrain::{mc_chunk::{MCActiveChunk, MCChunksListRender, MCPendingChunk}, MC_MAX_CHUNKS_PROCESS_PER_FRAME};

#[derive(Clone, Copy)]
struct Vec3C { x: f32, y: f32, z: f32 }
impl Vec3C {
    fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}
impl Hash for Vec3C {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.x.to_bits().hash(state);
        self.y.to_bits().hash(state);
        self.z.to_bits().hash(state);
    }
}
impl PartialEq for Vec3C {
    fn eq(&self, other: &Vec3C) -> bool {
        self.x == other.x && self.y == other.y && self.z == other.z
    }
}
impl Eq for Vec3C {}


#[derive(Component)]
pub struct MCProcessTaskManager(pub Task<CommandQueue>);
impl MCProcessTaskManager {
    /**
     * Process the chunks to generate the mesh data.
     */
    pub fn process_chunks(
        pending_chunks: Query<(Entity, &MCPendingChunk)>,
        mut commands: Commands
    ) {
        // If there are no chunks to process, return
        if pending_chunks.iter().count() == 0 {
            return;
        }

        // Process the chunks
        let thread_pool = AsyncComputeTaskPool::get();
        let mut chunks = vec![];
        for (entity, chunk) in pending_chunks.iter() {
            // Push the chunk to the list
            chunks.push(MCPendingChunk {
                index: chunk.index,
                raw_triangles: chunk.raw_triangles.clone(),
                triangles_counter: chunk.triangles_counter,
                points_gpu: chunk.points_gpu.clone(),
            });

            // Remove the pending chunk
            commands.entity(entity).despawn();
        }

        // Process the chunks
        let mut process_count = 0;
        for chunk in chunks {
            process_count += 1;
            if process_count >= MC_MAX_CHUNKS_PROCESS_PER_FRAME {
                break;
            }
            let task_entity = commands.spawn_empty().id();

            // Spawn a new task to process the chunk
            let task = thread_pool.spawn(async move {
                // Mesh data
                let mut vertices = Vec::new();
                let mut indices = Vec::new();

                // Extract the unique vertices and indices from the raw data
                let mut vertices_map = HashMap::new();
                let mut indices_counter = 0;
                for i in 0..chunk.triangles_counter as usize {
                    let triangle_normal = Vec3C::new(
                        chunk.raw_triangles[12*i + 3],
                        chunk.raw_triangles[12*i + 7],
                        chunk.raw_triangles[12*i + 11],
                    );
                    for j in 0..3 {
                        let vertex = Vec3C::new(
                            chunk.raw_triangles[12*i + j * 4],
                            chunk.raw_triangles[12*i + j * 4 + 1],
                            chunk.raw_triangles[12*i + j * 4 + 2],
                        );

                        // Add the vertex to the mesh data
                        indices.push(match vertices_map.get(&vertex) {
                            Some(vertex_index) => *vertex_index,
                            None => {
                                vertices.push(WVertex {
                                    position: [vertex.x, vertex.y, vertex.z],
                                    normal: [triangle_normal.x, triangle_normal.y, triangle_normal.z],
                                    uv: [0.0, 0.0],
                                });
                                vertices_map.insert(vertex, indices_counter);
                                indices_counter += 1;
                                indices_counter - 1
                            }
                        });
                    }
                }

                // Create the buffers
                let mut vertices_buffer = Buffer {
                    label: "".to_string(),
                    size: vertices.len() * std::mem::size_of::<WVertex>(),
                    usage: BufferUsage::VERTEX,
                    content: Some(bytemuck::cast_slice(&vertices).to_vec()),
                };
                let mut indices_buffer = Buffer {
                    label: "".to_string(),
                    size: indices.len() * std::mem::size_of::<u32>(),
                    usage: BufferUsage::INDEX,
                    content: Some(bytemuck::cast_slice(&indices).to_vec()),
                };

                // Return the mesh data
                let mut command_queue = CommandQueue::default();
                command_queue.push(move |world: &mut World| {
                    debug!("Registering chunk {:?} mesh data on the render thread with {} vertices and {} indices.", chunk.index, vertices.len(), indices.len());

                    // Get the chunk description
                    let desc = match world.get_resource::<MCChunksListRender>().unwrap().chunks.get(&chunk.index) {
                        Some(desc) => desc.clone(),
                        None => {
                            world.commands().entity(task_entity).despawn();
                            error!("The chunk description for chunk {:?} is missing. The chunk was probably removed during a parallel task.", chunk.index);
                            return;
                        }
                    };
                    vertices_buffer.label = format!("marching-cubes-vertices-{:?}", desc.index);
                    indices_buffer.label = format!("marching-cubes-indices-{:?}", desc.index);

                    // Insert the mesh data
                    let asset_server = world.get_resource_mut::<AssetServer>().unwrap();
                    let active_chunk = MCActiveChunk {
                        index: chunk.index,
                        vertices: asset_server.add(vertices_buffer),
                        indices: asset_server.add(indices_buffer),
                        indices_counter: indices.len() as u32,
                        points_gpu: chunk.points_gpu,
                    };
                    world.commands().spawn((active_chunk, desc));
                    world.commands().entity(task_entity).despawn();
                });
                command_queue
            });

            // Spawn the task entity
            commands.entity(task_entity).insert(MCProcessTaskManager(task));
        }
    }



    /**
     * Check if the tasks are done.
     * If so, run the command queue that will add the buffers and the chunk to the loading chunks.
     * If the task is done, despawn the entity.
     */
    pub fn handle_tasks(mut commands: Commands, mut tasks: Query<&mut MCProcessTaskManager>) {
        let mut process_count = 0;
        for mut task in &mut tasks {
            process_count += 1;
            if process_count >= MC_MAX_CHUNKS_PROCESS_PER_FRAME {
                break;
            }
            if let Some(mut commands_queue) = block_on(future::poll_once(&mut task.0)) {
                // Add the commands to the main thread for the next frame
                commands.append(&mut commands_queue);
            }
        }
    }
}
