use bevy::prelude::*;
use egui::Context;
use wde_renderer::core::RenderApp;

/// Resource to store the egui context
#[derive(Resource)]
pub struct EguiContext(pub Context);

/// Resource to store egui frame data between update and render passes
#[derive(Resource, Default)]
pub(crate) struct EguiFrameData {
    pub full_output: Option<egui::FullOutput>,
    pub paint_jobs: Option<Vec<egui::ClippedPrimitive>>,
    pub textures_delta: Option<egui::TexturesDelta>,
}

/// Plugin to add egui context and frame data resources
pub(crate) struct EguiContextPlugin;
impl Plugin for EguiContextPlugin {
    fn build(&self, app: &mut App) {
        // Initialize egui context (default)
        let ctx = Context::default();

        // Create and insert the egui context resource
        app
            .init_resource::<EguiFrameData>()
            .insert_resource(EguiContext(ctx));
        app.get_sub_app_mut(RenderApp).unwrap()
            .init_resource::<EguiFrameData>();
    }
}

/// System to tessellate egui shapes into paint jobs for rendering
/// 
/// # Arguments
/// * `ctx` - The egui context
/// * `frame_data` - The frame data resource to store paint jobs and textures delta
pub(crate) fn tessellate(ctx: Res<EguiContext>, mut frame_data: ResMut<EguiFrameData>) {
    let full_output = frame_data.full_output.take().expect("No frame data available for rendering");

    // handle_platform_output(full_output.platform_output);
    let paint_jobs = ctx.0.tessellate(full_output.shapes, full_output.pixels_per_point);
    frame_data.paint_jobs = Some(paint_jobs);
    frame_data.textures_delta = Some(full_output.textures_delta);
}
