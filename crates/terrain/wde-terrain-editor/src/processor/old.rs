use bevy::prelude::*;
use wde_terrain::prelude::{TILE_SIZE, Terrain};

use crate::paint::paint_manager::PaintManager;

pub struct PaintProcessorPlugin;
impl Plugin for PaintProcessorPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(PostUpdate, process_paint_commands);
    }
}


pub fn process_paint_commands(
    paint_manager: Res<PaintManager>,
    mut terrain_query: Query<&mut Terrain>
) {
    // Check if there is a terrain
    let mut terrain = match terrain_query.single_mut() {
        Ok(terrain) => terrain,
        Err(_) => return
    };

    // Get commands and chunks_pos
    let (commands, chunks_pos) = match (&paint_manager.commands, &paint_manager.commands_chunks) {
        (Some(commands), Some(chunks_pos)) => (commands.clone(), chunks_pos.clone()),
        _ => return
    };

    for (command, chunk_pos) in commands.into_iter().zip(chunks_pos.into_iter()) {
        println!("Processing paint command at world position {:?} with radius {} and strength {} and color {:?} and brush type {:?}", command.world_position, command.radius, command.strength, command.color, command.brush_type);
        // Get tile data for the current world position
        let data_as_u8 = match terrain.get_tile_data_for_chunk(chunk_pos, 1, 0) {
            Some(data) => data.clone(),
            None => continue
        };

        // Apply the brush effect in a small radius around the world position
        let mut data = data_as_u8.iter().map(|v| *v as f32 / 255.0).collect::<Vec<f32>>(); // Convert u8 data to f32 for processing
        let in_tile_pos = Vec2::new(
            (command.world_position.x - (chunk_pos.x as f32 * TILE_SIZE[0])) / TILE_SIZE[0] + 0.5,
            (command.world_position.z - (chunk_pos.y as f32 * TILE_SIZE[2])) / TILE_SIZE[2] + 0.5,
        );
        let radius = command.radius / TILE_SIZE[0]; // Brush size in tile space
        let ss = (data.len() as f32 / 4.0).sqrt(); // Subdivision size of the tile (divided by 4 for RGBA)
        for x in 0..ss as usize {
            for y in 0..ss as usize {
                let tile_pos = Vec2::new(x as f32 / ss, y as f32 / ss);
                let distance = tile_pos.distance(in_tile_pos);
                if distance < radius {
                    let strength = (1.0 - (distance / radius)) * command.strength;
                    let idx = (y * ss as usize + x) * 4; // RGBA channels
                    data[idx] = command.color[0] * strength + data[idx] * (1.0 - strength);
                    data[idx + 1] = command.color[1] * strength + data[idx + 1] * (1.0 - strength);
                    data[idx + 2] = command.color[2] * strength + data[idx + 2] * (1.0 - strength);
                }
            }
        }
        let new_data_as_u8 = data.iter().map(|v| (v.clamp(0.0, 1.0) * 255.0) as u8).collect::<Vec<u8>>(); // Convert modified f32 data back to u8

        // Write the modified data back to the terrain
        terrain.set_tile_data_for_chunk(chunk_pos, 1, 0, new_data_as_u8);
    }
}