use bevy::{log::{Level, tracing::event}, prelude::*};

use crate::terrain::{mc_chunk::{MCChunkDescription, MCChunksListMain}, TerrainSpawner};

pub struct MarchingCubesSpawner;
impl MarchingCubesSpawner {
    /**
     * Manage the chunks creation / deletion.
     * This create a task for each chunk to generate the mesh.
     * This will then generate the points, vertices and indices buffers, and add the chunk to the loading chunks.
     */
    pub fn manage_chunks(
        mut commands: Commands,
        chunks_list: Res<MCChunksListMain>,
        chunk_spawner_query: Query<(&Transform, &TerrainSpawner), Changed<Transform>>
    ) {
        // Get the terrain spawner
        let (cs_transform, cs) = match chunk_spawner_query.single() {
            Ok((transform, chunk_spawner)) => (transform, chunk_spawner),
            Err(_) => {
                if chunk_spawner_query.iter().count() > 1 {
                    error!("There should be only one terrain spawner in the scene.");
                }
                return;
            }
        };

        // Compute the list of chunks that should be spawned around the spawner
        let mut new_chunks = Vec::new();
        let mut current_chunks = chunks_list.current_chunks.clone();
        let mut delete_chunks = chunks_list.current_chunks.clone();
        for i in -cs.chunk_radius_count..cs.chunk_radius_count {
            for k in -cs.chunk_radius_count..cs.chunk_radius_count {
                // Check if in the circle
                if i * i + k * k > cs.chunk_radius_count * cs.chunk_radius_count {
                    continue;
                }

                // Compute the world index of the chunk
                let chunk_global_index = (
                    (cs_transform.translation.x / cs.chunk_length[0] + 0.5).round() as i32 + i,
                    0,
                    (cs_transform.translation.z / cs.chunk_length[2] + 0.5).round() as i32 + k
                );

                // Check if the chunk is already spawned
                if current_chunks.contains_key(&chunk_global_index) {
                    delete_chunks.remove(&chunk_global_index);
                    continue;
                }

                // Spawn the chunk
                let translation = Vec3::new(
                    chunk_global_index.0 as f32 * cs.chunk_length[0],
                    0.0,
                    chunk_global_index.2 as f32 * cs.chunk_length[2]
                );
                let desc = MCChunkDescription {
                    index: chunk_global_index,
                    translation,
                    length: cs.chunk_length.into(),
                    sub_count: cs.chunk_sub_count.into(),
                    iso_level: cs.iso_level
                };
                event!(Level::TRACE, "Spawn chunk at index {:?}: {:?}", chunk_global_index, desc.clone());
                current_chunks.insert(chunk_global_index, chunk_global_index);
                new_chunks.push((chunk_global_index, desc));
            }
        }

        // Remove the chunks that should be deleted
        for chunk_index in delete_chunks.keys() {
            event!(Level::TRACE, "Delete chunk at index {:?}.", chunk_index);
            current_chunks.remove(chunk_index);
        }

        // Update the chunks list
        commands.insert_resource(MCChunksListMain {
            current_chunks,
            new_chunks,
            delete_chunks: delete_chunks.keys().cloned().collect()
        });
    }
}