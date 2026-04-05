use wde_logger::prelude::*;

use bevy::{asset::io::embedded::GetAssetServer, prelude::*};
use std::collections::HashSet;
use wde_renderer::prelude::*;
use wde_terrain::prelude::ChunkPos;

use crate::{paint::brush::PaintMode, processor::ExtractedPaintCommands};

// The maximum number of commands that can be stored by render frame.
const MAX_COMMANDS: usize = 1000;

pub struct ComputeCommandsBufferPlugin;
impl Plugin for ComputeCommandsBufferPlugin {
    fn build(&self, app: &mut App) {
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .init_resource::<CommandsBuffer>()
            .add_systems(
                Render,
                build_commands_bind_group.in_set(RenderSet::BindGroups)
            )
            .add_systems(Render, update_commands_buffer.in_set(RenderSet::Prepare));
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CommandDescription {
    pub world_position: [f32; 2],
    pub radius: f32,
    pub strength: f32,
    pub color: [f32; 4],
    pub brush_type: f32,
    pub _padding: [f32; 3]
}

#[derive(Resource)]
pub struct CommandsBuffer {
    pub commands_buffer: Handle<Buffer>,
    pub commands_count: usize,
    pub dirty_chunks: HashSet<ChunkPos>,

    pub layout: BindGroupLayout,
    pub layout_built: WgpuBindGroupLayout,
    pub bind_group: Option<BindGroup>
}
impl FromWorld for CommandsBuffer {
    fn from_world(world: &mut World) -> Self {
        let render_instance = world.get_resource::<RenderInstance>().unwrap();

        // Create the buffer
        let commands_buffer = world.get_asset_server().add(Buffer {
            label: "commands-buffer".to_string(),
            size: std::mem::size_of::<CommandDescription>() * MAX_COMMANDS,
            usage: BufferUsage::STORAGE | BufferUsage::COPY_DST,
            content: None
        });

        // Create the terrain layout
        let layout = BindGroupLayout::new("commands", |builder| {
            builder.add_buffer(
                0,
                ShaderStages::COMPUTE,
                BufferBindingType::Storage { read_only: true }
            );
        });
        let layout_built = layout.build(&render_instance.0.read().unwrap()).unwrap();

        CommandsBuffer {
            commands_buffer,
            commands_count: 0,
            layout,
            layout_built,
            bind_group: None,
            dirty_chunks: HashSet::new()
        }
    }
}

fn build_commands_bind_group(
    render_instance: Res<RenderInstance>,
    mut commands_buffer: ResMut<CommandsBuffer>,
    buffers: Res<RenderAssets<GpuBuffer>>
) {
    // Check if the bind group is already created
    if commands_buffer.bind_group.is_some() {
        return;
    }

    // Create the bind group
    if let Some(buffer) = buffers.get(&commands_buffer.commands_buffer) {
        let render_instance = render_instance.0.read().unwrap();
        let bind_group = BindGroupBuilder::build(
            "commands",
            &render_instance,
            &commands_buffer.layout_built,
            &vec![BindGroupBuilder::buffer(0, &buffer.buffer)]
        )
        .unwrap();
        commands_buffer.bind_group = Some(bind_group);
    }
}

// System to update the terrain tiles buffer with the current visible tiles
fn update_commands_buffer(
    render_instance: Res<RenderInstance>,
    mut commands_buffer: ResMut<CommandsBuffer>,
    buffers: Res<RenderAssets<GpuBuffer>>,
    mut extracted_commands: ResMut<ExtractedPaintCommands>
) {
    let _span = info_span!("update_commands_buffer").entered();
    // Check if the bind group is already created or if there are no commands to upload
    if extracted_commands.commands.is_empty() || commands_buffer.bind_group.is_none() {
        // Clear the dirty chunks and commands in the command buffer
        commands_buffer.dirty_chunks.clear();
        commands_buffer.commands_count = 0;
        return;
    }

    // Get the buffer
    let commands_bf = match buffers.get(&commands_buffer.commands_buffer) {
        Some(buffer) => buffer,
        None => return
    };

    // Prepare the data
    let data: Vec<CommandDescription> = extracted_commands
        .commands
        .iter()
        .map(|command| CommandDescription {
            world_position: [command.world_position.x, command.world_position.z],
            radius: command.radius,
            strength: command.strength,
            color: command.color,
            brush_type: match command.paint_mode {
                PaintMode::Paint => 0.0,
                PaintMode::Erase => 1.0,

                PaintMode::Raise => 2.0,
                PaintMode::Lower => 3.0,
                PaintMode::Smooth => 4.0,
                PaintMode::Flatten => 5.0
            },
            _padding: [0.0; 3]
        })
        .collect();

    // Update the buffer and clear the commands
    let render_instance = render_instance.0.read().unwrap();
    commands_bf
        .buffer
        .write(&render_instance, bytemuck::cast_slice(&data), 0);

    // Update the dirty chunks
    commands_buffer.dirty_chunks = extracted_commands.dirty_chunks.take().unwrap_or_default();
    commands_buffer.commands_count = data.len();

    // Make sure to clear extracted commands
    extracted_commands.commands.clear();
    extracted_commands.dirty_chunks = None;
}
