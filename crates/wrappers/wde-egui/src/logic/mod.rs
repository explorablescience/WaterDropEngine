use bevy::prelude::*;

use crate::egui::{egui_context::{EguiContext, EguiFrameData, tessellate}, egui_inputs::{EguiInputs, handle_input}};

/// Plugin to add systems for updating egui and tessellating paint jobs
pub struct EguiRenderPlugin;
impl Plugin for EguiRenderPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(PreUpdate, (handle_input, preupdate).chain())
            .add_systems(Update, update)
            .add_systems(PostUpdate, (postupdate, tessellate).chain());
    }
}

/// System to perform any pre-update logic for egui - Start the render pass
fn preupdate(ctx: Res<EguiContext>, mut egui_inputs: ResMut<EguiInputs>) {
    let raw_input = egui_inputs.0.take();
    ctx.0.begin_pass(raw_input);
}

/// System to update the egui context with new input and generate frame output
fn update(ctx: Res<EguiContext>) {
    egui::Window::new("winit + egui + wgpu says hello!")
        .resizable(true)
        .vscroll(true)
        .default_open(false)
        .show(&ctx.0, |ui| {
            ui.label("Label!");
            if ui.button("Button!").clicked() {
                println!("boom!")
            }
        });
}

/// System to perform any post-update logic for egui - End the render pass
fn postupdate(ctx: Res<EguiContext>, mut frame_data: ResMut<EguiFrameData>) {
    let full_output = ctx.0.end_pass();
    frame_data.full_output = Some(full_output);
}
