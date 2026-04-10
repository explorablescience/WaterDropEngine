use wde_logger::prelude::*;

use bevy::{ecs::system::SystemParamItem, prelude::*};
use std::collections::HashSet;
use wde_renderer::prelude::*;
use wde_terrain::prelude::ChunkPos;

use crate::{paint::brush::PaintMode, processor::ExtractedPaintCommands};

// The maximum number of commands that can be stored by render frame.
const MAX_COMMANDS: usize = 1000;

pub struct ComputeCommandsBufferPlugin;
impl Plugin for ComputeCommandsBufferPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((RenderDataRegisterPlugin::<CommandsBuffer>::default(), RenderBindingRegisterPlugin::<CommandsBufferBinding>::default()));
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .init_resource::<CommandsBufferDescription>()
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

#[derive(Resource, Default)]
pub struct CommandsBufferDescription {
    pub commands_count: usize,
    pub dirty_chunks: HashSet<ChunkPos>
}


#[derive(Asset, Clone, Debug, TypePath, Default)]
pub struct CommandsBufferBinding;
impl RenderBinding for CommandsBufferBinding {
    type Params = SRenderData<CommandsBuffer>;

    fn describe(&mut self, command_buffer: &SystemParamItem<Self::Params>, builder: &mut RenderBindingBuilder) {
        builder.add_buffer(command_buffer, CommandsBuffer::BINDING);
    }

    fn label(&self) -> &str {
        "commands-buffer-binding"
    }
}

#[derive(Asset, Clone, Debug, TypePath, Default)]
pub struct CommandsBuffer;
impl CommandsBuffer {
    pub const BINDING: u32 = 0;
}
impl RenderData for CommandsBuffer {
    type Params = ();

    fn describe(_params: &mut SystemParamItem<Self::Params>, builder: &mut RenderDataBuilder) {
        builder.add_buffer(
            Self::BINDING,
            Buffer {
                label: "commands-buffer".to_string(),
                size: std::mem::size_of::<CommandDescription>() * MAX_COMMANDS,
                usage: BufferUsage::STORAGE | BufferUsage::COPY_DST,
                content: None
            }
        );
    }
}

// System to update the terrain tiles buffer with the current visible tiles
fn update_commands_buffer(
    render_instance: Res<RenderInstance>,
    commands_buffer: ResRenderData<CommandsBuffer>,
    mut commands_buffer_desc: ResMut<CommandsBufferDescription>,
    buffers: Res<RenderAssets<GpuBuffer>>,
    mut extracted_commands: ResMut<ExtractedPaintCommands>,
    mut is_not_first: Local<bool>
) {
    let _span = info_span!("update_commands_buffer").entered();
    // Check if the bind group is already created or if there are no commands to upload
    if extracted_commands.commands.is_empty() || !*is_not_first {
        // Clear the dirty chunks and commands in the command buffer
        commands_buffer_desc.dirty_chunks.clear();
        commands_buffer_desc.commands_count = 0;
        *is_not_first = true;
        return;
    }

    // Get the buffer
    let commands_buffer = match commands_buffer.iter().next().and_then(|(_, b)| buffers.get(&b.get_buffer(CommandsBuffer::BINDING).unwrap())) {
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
    commands_buffer
        .buffer
        .write(&render_instance, bytemuck::cast_slice(&data), 0);

    // Update the dirty chunks
    commands_buffer_desc.dirty_chunks = extracted_commands.dirty_chunks.take().unwrap_or_default();
    commands_buffer_desc.commands_count = data.len();

    // Make sure to clear extracted commands
    extracted_commands.commands.clear();
    extracted_commands.dirty_chunks = None;
}
