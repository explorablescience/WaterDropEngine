use std::sync::{Arc, RwLock};

use bevy::{input::{ButtonState, keyboard::KeyboardInput, mouse::{self, MouseButtonInput}, prelude::*}, prelude::*};
use egui::Context;
use egui_wgpu::{RendererOptions, ScreenDescriptor};
use wde_renderer::prelude::*;

pub mod prelude {
    pub use crate::{EguiContext, EguiFrameData, EguiPlugin, EguiInputs};
}

#[derive(Resource)]
pub struct EguiContext {
    pub ctx: Context,
}

#[derive(Resource, Default)]
pub struct EguiFrameData {
    full_output: Option<egui::FullOutput>,
    paint_jobs: Option<Vec<egui::ClippedPrimitive>>,
    textures_delta: Option<egui::TexturesDelta>,
}

#[derive(Resource)]
pub struct EguiRenderer {
    pub renderer: Arc<RwLock<egui_wgpu::Renderer>>,
}

#[derive(Resource, Default)]
pub struct EguiInputs(pub egui::RawInput);

pub struct EguiPlugin;
impl Plugin for EguiPlugin {
    fn build(&self, app: &mut App) {
        let ctx = Context::default();
        app
            .init_resource::<EguiFrameData>()
            .init_resource::<EguiInputs>()
            .insert_resource(EguiContext { ctx })
            .add_systems(Update, (handle_input, update, tessellate).chain());

        app.get_sub_app_mut(RenderApp).unwrap()
            .init_resource::<EguiFrameData>();

        // Add render pass
        let mut render_graph = app.get_sub_app_mut(RenderApp).unwrap()
            .world_mut().get_resource_mut::<RenderGraph>().unwrap();
        render_graph.add_pass::<EguiRenderPass>(200);
    }

    fn finish(&self, app: &mut App) {
        // Get WGPU device
        let instance = {
            let render_app = app.get_sub_app_mut(RenderApp);
            let render_app = render_app.unwrap();
            render_app.world().resource::<RenderInstance>()
        };

        // Create egui render pass
        let egui_rpass = egui_wgpu::Renderer::new(&instance.0.read().unwrap().device, wgpu::TextureFormat::Bgra8UnormSrgb, RendererOptions::default());
        app.get_sub_app_mut(RenderApp).unwrap()
            .insert_resource(EguiRenderer { renderer: Arc::new(RwLock::new(egui_rpass)) });
    }
}

fn handle_input(
    mut mouse_button_events: MessageReader<MouseButtonInput>,
    // mut mouse_motion_events: MessageReader<MouseMotion>,
    // mut mouse_wheel_events: MessageReader<MouseWheel>,
    mut keyboard_events: MessageReader<KeyboardInput>,
    windows: Query<&Window>,
    mut egui_inputs: ResMut<EguiInputs>,
) {
    let mut raw_input = egui::RawInput::default();

    // Get mouse position
    let window = windows.iter().next().unwrap();
    let mouse_position = window.cursor_position().map(|pos| egui::Pos2 { x: pos.x, y: pos.y });

    // Handle mouse buttons
    for event in mouse_button_events.read() {
        let pointer_button = match event.button {
            MouseButton::Left => Some(egui::PointerButton::Primary),
            MouseButton::Right => Some(egui::PointerButton::Secondary),
            MouseButton::Middle => Some(egui::PointerButton::Middle),
            _ => None,
        };

        if let Some(button) = pointer_button {
            match event.state {
                ButtonState::Pressed => {
                    raw_input.events.push(egui::Event::PointerButton {
                        pos: mouse_position.unwrap_or_default(),
                        button,
                        pressed: true,
                        modifiers: egui::Modifiers::default(),
                    });
                }
                ButtonState::Released => {
                    raw_input.events.push(egui::Event::PointerButton {
                        pos: mouse_position.unwrap_or_default(),
                        button,
                        pressed: false,
                        modifiers: egui::Modifiers::default(),
                    });
                }
            }
        }
    }

    // Handle mouse wheel
    // for event in mouse_wheel_events.read() {
    //     let delta = match event.unit {
    //         MouseScrollUnit::Line => event.y * 50.0,
    //         MouseScrollUnit::Pixel => event.y,
    //     };
    //     raw_input.events.push(egui::Event::Scroll(egui::Vec2::new(0.0, delta)));
    // }

    // Handle keyboard input
    for event in keyboard_events.read() {
        if event.state == ButtonState::Pressed {
            if let Some(key) = bevy_to_egui_key(event.key_code) {
                raw_input.events.push(egui::Event::Key {
                    key,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::default(),
                    physical_key: None,
                });
            }
        } else if event.state == ButtonState::Released
            && let Some(key) = bevy_to_egui_key(event.key_code) {
                raw_input.events.push(egui::Event::Key {
                    key,
                    pressed: false,
                    repeat: false,
                    modifiers: egui::Modifiers::default(),
                    physical_key: None,
                });
            }
    }

    // Handle text input
    // for event in character_events.read() {
    //     raw_input.events.push(egui::Event::Text(event.char.to_string()));
    // }

    egui_inputs.0 = raw_input;
}

fn bevy_to_egui_key(key: KeyCode) -> Option<egui::Key> {
    match key {
        KeyCode::ArrowUp => Some(egui::Key::ArrowUp),
        KeyCode::ArrowDown => Some(egui::Key::ArrowDown),
        KeyCode::ArrowLeft => Some(egui::Key::ArrowLeft),
        KeyCode::ArrowRight => Some(egui::Key::ArrowRight),
        KeyCode::Escape => Some(egui::Key::Escape),
        KeyCode::Tab => Some(egui::Key::Tab),
        KeyCode::Backspace => Some(egui::Key::Backspace),
        KeyCode::Enter => Some(egui::Key::Enter),
        KeyCode::Space => Some(egui::Key::Space),
        KeyCode::Delete => Some(egui::Key::Delete),
        KeyCode::Home => Some(egui::Key::Home),
        KeyCode::End => Some(egui::Key::End),
        KeyCode::PageUp => Some(egui::Key::PageUp),
        KeyCode::PageDown => Some(egui::Key::PageDown),
        _ => None,
    }
}

fn update(ctx: Res<EguiContext>, mut frame_data: ResMut<EguiFrameData>, mut egui_inputs: ResMut<EguiInputs>) {
    let raw_input = egui_inputs.0.take();
    let ctx = &ctx.ctx;
    let full_output = ctx.run(raw_input, |ctx| {
        egui::Window::new("winit + egui + wgpu says hello!")
            .resizable(true)
            .vscroll(true)
            .default_open(false)
            .show(ctx, |ui| {
                ui.label("Label!");
                if ui.button("Button!").clicked() {
                    println!("boom!")
                }
            });
    });
    frame_data.full_output = Some(full_output);
}

fn tessellate(ctx: Res<EguiContext>, mut frame_data: ResMut<EguiFrameData>) {
    let full_output = frame_data.full_output.take().expect("No frame data available for rendering");
    let ctx = &ctx.ctx;

    // handle_platform_output(full_output.platform_output);
    let paint_jobs = ctx.tessellate(full_output.shapes, full_output.pixels_per_point);
    frame_data.paint_jobs = Some(paint_jobs);
    frame_data.textures_delta = Some(full_output.textures_delta);
}


#[derive(Default)]
pub struct EguiRenderPass;
impl RenderPass for EguiRenderPass {
    fn extract(&self, main_world: &mut World, render_world: &mut World) {
        let frame_data_main = main_world.resource::<EguiFrameData>();
        let mut frame_data_render = render_world.resource_mut::<EguiFrameData>();
        frame_data_render.paint_jobs = frame_data_main.paint_jobs.clone();
        frame_data_render.textures_delta = frame_data_main.textures_delta.clone();
    }

    fn render(&self, render_world: &mut World) {
        // Create a render pass
        let instance = render_world.resource::<RenderInstance>();
        let instance_guard = instance.0.read().unwrap(); // keep the lock alive for both device and queue
        let device = &instance_guard.device;
        let queue = &instance_guard.queue;
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("egui_render_encoder") });
        {
            let render_pass: wgpu::RenderPass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &render_world.get_resource::<SwapchainFrame>().unwrap().data.as_ref().unwrap().view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
            let mut render_pass = render_pass.forget_lifetime();

            // Get swapchain size
            let swapchain_descriptor = instance_guard.surface_config.clone().unwrap();

            let egui_renderer = render_world.resource::<EguiRenderer>();
            let frame_data = render_world.resource::<EguiFrameData>();
            let screen_descriptor = ScreenDescriptor {
                pixels_per_point: 1.0,
                size_in_pixels: [swapchain_descriptor.width, swapchain_descriptor.height],
            };

            // Update egui textures
            let textures_delta = frame_data.textures_delta.clone().unwrap();
            for (id, image_delta) in &textures_delta.set {
                egui_renderer.renderer.write().unwrap().update_texture(device, queue, *id, image_delta);
            }
            for id in &textures_delta.free {
                egui_renderer.renderer.write().unwrap().free_texture(id);
            }
            
            // Draw
            egui_renderer.renderer.write().unwrap().update_buffers(device, queue, &mut encoder, frame_data.paint_jobs.as_ref().unwrap(), &screen_descriptor);
            egui_renderer.renderer.read().unwrap().render(&mut render_pass, frame_data.paint_jobs.as_ref().unwrap(), &screen_descriptor);
        }

        // Submit commands
        queue.submit(Some(encoder.finish()));
    }
}
