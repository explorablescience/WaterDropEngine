use std::{
    collections::{BTreeMap, VecDeque},
    sync::{Mutex, OnceLock},
    time::SystemTime
};

use tracing::{
    Level,
    field::{Field, Visit}
};
use tracing_subscriber::{Layer, layer::Context};

const EDITOR_LOG_CAPACITY: usize = 4_000;

#[derive(Clone, Debug)]
pub struct EditorLogEntry {
    pub timestamp: SystemTime,
    pub level: Level,
    pub target: String,
    pub message: String
}

static EDITOR_LOG_BUFFER: OnceLock<Mutex<VecDeque<EditorLogEntry>>> = OnceLock::new();

fn editor_log_buffer() -> &'static Mutex<VecDeque<EditorLogEntry>> {
    EDITOR_LOG_BUFFER.get_or_init(|| Mutex::new(VecDeque::with_capacity(EDITOR_LOG_CAPACITY)))
}
fn push_editor_log(entry: EditorLogEntry) {
    let mut logs = editor_log_buffer()
        .lock()
        .expect("editor log buffer should not be poisoned");
    if logs.len() >= EDITOR_LOG_CAPACITY {
        logs.pop_front();
    }
    logs.push_back(entry);
}
pub fn editor_logs_snapshot() -> Vec<EditorLogEntry> {
    editor_log_buffer()
        .lock()
        .expect("editor log buffer should not be poisoned")
        .iter()
        .cloned()
        .collect()
}
pub fn editor_logs_clear() {
    editor_log_buffer()
        .lock()
        .expect("editor log buffer should not be poisoned")
        .clear();
}

#[derive(Default)]
pub(crate) struct EditorLogVisitor {
    message: Option<String>,
    fields: BTreeMap<String, String>
}
impl Visit for EditorLogVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn core::fmt::Debug) {
        self.record(field, format!("{value:?}"));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.record(field, value.to_string());
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.record(field, value.to_string());
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.record(field, value.to_string());
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.record(field, value.to_string());
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        self.record(field, value.to_string());
    }
}
impl EditorLogVisitor {
    fn record(&mut self, field: &Field, value: String) {
        if field.name() == "message" {
            self.message = Some(value.trim_matches('"').to_string());
        } else {
            self.fields.insert(field.name().to_string(), value);
        }
    }

    fn build_message(self) -> String {
        let mut message = self.message.unwrap_or_default();
        if !self.fields.is_empty() {
            let trailing = self
                .fields
                .into_iter()
                .map(|(key, value)| format!("{key}={value}"))
                .collect::<Vec<_>>()
                .join(" ");
            if message.is_empty() {
                message = trailing;
            } else {
                message.push(' ');
                message.push_str(&trailing);
            }
        }
        if message.is_empty() {
            "<empty log event>".to_string()
        } else {
            message
        }
    }
}

pub(crate) struct EditorLogLayer;
impl<S> Layer<S> for EditorLogLayer
where
    S: tracing::Subscriber
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let metadata = event.metadata();
        let mut visitor = EditorLogVisitor::default();
        event.record(&mut visitor);

        push_editor_log(EditorLogEntry {
            timestamp: SystemTime::now(),
            level: *metadata.level(),
            target: metadata.target().to_string(),
            message: visitor.build_message()
        });
    }
}
