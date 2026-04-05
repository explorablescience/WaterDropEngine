use std::{
    collections::{BTreeMap, VecDeque},
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH}
};

use tracing::{
    Level,
    field::{Field, Visit}
};
use tracing_subscriber::{Layer, layer::Context};

const PANIC_REPORT_LOG_CAPACITY: usize = 8_000;

#[derive(Clone, Debug)]
struct PanicReportLogEntry {
    timestamp_unix_ms: u128,
    level: Level,
    target: String,
    message: String
}

static PANIC_REPORT_LOG_BUFFER: OnceLock<Mutex<VecDeque<PanicReportLogEntry>>> = OnceLock::new();

fn panic_report_log_buffer() -> &'static Mutex<VecDeque<PanicReportLogEntry>> {
    PANIC_REPORT_LOG_BUFFER
        .get_or_init(|| Mutex::new(VecDeque::with_capacity(PANIC_REPORT_LOG_CAPACITY)))
}

fn push_panic_report_log(entry: PanicReportLogEntry) {
    let mut logs = panic_report_log_buffer()
        .lock()
        .expect("panic report log buffer should not be poisoned");
    if logs.len() >= PANIC_REPORT_LOG_CAPACITY {
        logs.pop_front();
    }
    logs.push_back(entry);
}

pub fn panic_report_logs_snapshot() -> Vec<String> {
    panic_report_log_buffer()
        .lock()
        .expect("panic report log buffer should not be poisoned")
        .iter()
        .map(|entry| {
            format!(
                "[{}ms] {} {}: {}",
                entry.timestamp_unix_ms, entry.level, entry.target, entry.message
            )
        })
        .collect()
}

#[derive(Default)]
struct PanicReportLogVisitor {
    message: Option<String>,
    fields: BTreeMap<String, String>
}
impl Visit for PanicReportLogVisitor {
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
impl PanicReportLogVisitor {
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

pub(crate) struct PanicReportLogLayer;
impl<S> Layer<S> for PanicReportLogLayer
where
    S: tracing::Subscriber
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let metadata = event.metadata();
        let mut visitor = PanicReportLogVisitor::default();
        event.record(&mut visitor);

        let timestamp_unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_millis());

        push_panic_report_log(PanicReportLogEntry {
            timestamp_unix_ms,
            level: *metadata.level(),
            target: metadata.target().to_string(),
            message: visitor.build_message()
        });
    }
}
