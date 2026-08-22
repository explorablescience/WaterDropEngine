use bevy::prelude::*;

use crate::egui::{
    egui_context::{EguiContext, EguiFrameData, tessellate},
    egui_inputs::{EguiInputs, clear_egui_inputs, handle_input}
};

/// System set containing [`handle_input`], the system that reads raw window/input events into
/// egui's [`egui::RawInput`] for this frame.
///
/// Other `PreUpdate` systems that need to consume or clear the same raw Bevy input resources
/// (e.g. to keep game systems from also reacting to input meant for a specific egui widget)
/// should run after this set, so `handle_input` has already forwarded the events to egui.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct EguiInputSet;

/// Plugin to add systems for updating egui and tessellating paint jobs
pub struct EguiRenderPlugin;
impl Plugin for EguiRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreUpdate,
            (handle_input.in_set(EguiInputSet), preupdate, clear_egui_inputs).chain()
        )
        .add_systems(PostUpdate, (postupdate, tessellate).chain());
    }
}

/// System to perform any pre-update logic for egui - Start the render pass
fn preupdate(ctx: Res<EguiContext>, mut egui_inputs: ResMut<EguiInputs>) {
    let raw_input = egui_inputs.0.take();
    ctx.0.begin_pass(raw_input);
}

/// System to perform any post-update logic for egui - End the render pass
fn postupdate(ctx: Res<EguiContext>, mut frame_data: ResMut<EguiFrameData>) {
    let full_output = ctx.0.end_pass();
    frame_data.full_output = Some(full_output);
}
