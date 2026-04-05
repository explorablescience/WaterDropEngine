use bevy::prelude::*;
use wde_egui::prelude::*;
use wde_logger::{
    Level,
    editor_layer::{editor_logs_clear, editor_logs_snapshot}
};

use crate::ui::UIMenu;

pub struct LogsPanelPlugin;
impl Plugin for LogsPanelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LogFilters>()
            .add_systems(Startup, init_ui)
            .add_systems(Update, draw_logs_panel);
    }
}

#[derive(Resource, Debug)]
struct LogFilters {
    error: bool,
    warn: bool,
    info: bool,
    debug: bool,
    trace: bool
}
impl Default for LogFilters {
    fn default() -> Self {
        Self {
            error: true,
            warn: true,
            info: true,
            debug: false,
            trace: false
        }
    }
}
impl LogFilters {
    fn allows(&self, level: Level) -> bool {
        match level {
            Level::ERROR => self.error,
            Level::WARN => self.warn,
            Level::INFO => self.info,
            Level::DEBUG => self.debug,
            Level::TRACE => self.trace
        }
    }
}

fn init_ui(mut ui_menu: ResMut<UIMenu>) {
    ui_menu.push("Engine/Logs");
}

fn draw_logs_panel(
    ctx: Res<EguiContext>,
    mut ui_menu: ResMut<UIMenu>,
    mut filters: ResMut<LogFilters>
) {
    if !ui_menu.is_clicked("Engine/Logs") {
        return;
    }

    egui::Window::new("Logs")
        .default_size(egui::vec2(800.0, 400.0))
        .open(ui_menu.clicked_mut("Engine/Logs").unwrap_or(&mut false))
        .show(&ctx.0, |ui| {
            ui.horizontal(|ui| {
                ui.label("Filter:");
                ui.checkbox(&mut filters.error, "Error");
                ui.checkbox(&mut filters.warn, "Warn");
                ui.checkbox(&mut filters.info, "Info");
                ui.checkbox(&mut filters.debug, "Debug");
                ui.checkbox(&mut filters.trace, "Trace");

                if ui.button("Clear").clicked() {
                    editor_logs_clear();
                }
            });

            ui.separator();

            let logs = editor_logs_snapshot();
            egui::ScrollArea::vertical()
                .stick_to_bottom(true)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for log in logs.into_iter().filter(|entry| filters.allows(entry.level)) {
                        let color = level_color(log.level);
                        let time = format_timestamp(log.timestamp);
                        ui.horizontal_wrapped(|ui| {
                            ui.colored_label(color, format!("[{time}]"));
                            ui.colored_label(color, format!("[{:<5}]", level_label(log.level)));
                            ui.monospace(format!("{}:", log.target));
                            ui.label(log.message);
                        });
                    }
                });
        });
}

fn level_label(level: Level) -> &'static str {
    match level {
        Level::ERROR => "ERROR",
        Level::WARN => "WARN",
        Level::INFO => "INFO",
        Level::DEBUG => "DEBUG",
        Level::TRACE => "TRACE"
    }
}

fn level_color(level: Level) -> egui::Color32 {
    match level {
        Level::ERROR => egui::Color32::from_rgb(220, 70, 70),
        Level::WARN => egui::Color32::from_rgb(240, 180, 50),
        Level::INFO => egui::Color32::from_rgb(130, 170, 255),
        Level::DEBUG => egui::Color32::from_rgb(180, 180, 180),
        Level::TRACE => egui::Color32::from_rgb(120, 120, 120)
    }
}

fn format_timestamp(timestamp: std::time::SystemTime) -> String {
    match timestamp.duration_since(std::time::UNIX_EPOCH) {
        Ok(duration) => {
            let seconds = duration.as_secs() % 86_400;
            let hours = seconds / 3_600;
            let minutes = (seconds % 3_600) / 60;
            let secs = seconds % 60;
            let millis = duration.subsec_millis();
            format!("{hours:02}:{minutes:02}:{secs:02}.{millis:03}")
        }
        Err(_) => "00:00:00.000".to_string()
    }
}
