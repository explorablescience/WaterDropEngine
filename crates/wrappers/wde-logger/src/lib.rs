//! Logging functions and configuration for WaterDropEngine using [`tracing`](https://docs.rs/tracing).
//!
//! The logging behavior can be configured using the [`LogPlugin`], which allows you to set the log level, filter logs using `EnvFilter` syntax, and add custom layers to the tracing subscriber. The `RUST_LOG` environment variable can also be used to override plugin settings and configure log filtering.
//!
//! This crate provides multiple macros for logging, including:
//! - [`trace!()`](crate::trace) - Verbose tracing information, typically only useful for debugging.
//! - [`debug!()`](crate::debug) - Debug information, useful for development and debugging.
//! - [`info!()`](crate::info) - General information about the application's operation.
//! - [`warn!()`](crate::warn) - Important warnings that may indicate potential issues.
//! - [`error!()`](crate::error) - Critical errors that indicate failures in the application.
//!
//! Each of these macros also has a corresponding `_once` variant (e.g. [`info_once!()`](crate::info_once)) that will only log the message once per call site, which is useful for logging within systems that are called every frame without spamming the logs.
//!
//! Lastly, span tracing is supported using the `*_span!()` macros (e.g. [`info_span!()`](crate::info_span)) which can be used to create spans for better structuring of logs and performance profiling.

pub mod editor_layer;
mod panic_handler;
mod panic_report_layer;
mod puffin_layer;

extern crate alloc;

use core::error::Error;

mod once;
pub use once::OnceFlag;

#[doc(hidden)]
pub mod prelude {
    pub use crate::LogLevel;
    pub use crate::{debug_once, error_once, info_once, trace_once, warn_once};
    pub use tracing::event;
    pub use tracing::{
        debug, debug_span, error, error_span, info, info_span, trace, trace_span, warn, warn_span
    };
}

pub use tracing::{
    self, Level as LogLevel, debug, debug_span, error, error_span, info, info_span, trace,
    trace_span, warn, warn_span
};

use bevy::prelude::*;
use tracing_log::LogTracer;
use tracing_subscriber::{
    EnvFilter, Layer,
    filter::{FromEnvError, ParseError},
    layer::Layered,
    prelude::*,
    registry::Registry
};

// Store the guard for the non-blocking file appender to ensure logs are flushed on drop and to prevent it from being dropped while the subscriber is still active.
#[allow(dead_code)]
#[derive(Resource)]
struct LoggerGuard(tracing_appender::non_blocking::WorkerGuard);

#[cfg(feature = "puffin")]
use crate::puffin_layer::PuffinLayer;

/// A boxed [`Layer`] that can be used with [`LogPlugin::custom_layer`].
type BoxedLayer = Box<dyn Layer<Registry> + Send + Sync + 'static>;
#[cfg(feature = "tracing")]
type BaseSubscriber =
    Layered<EnvFilter, Layered<Option<Box<dyn Layer<Registry> + Send + Sync>>, Registry>>;
#[cfg(feature = "tracing")]
type PreFmtSubscriber = Layered<tracing_error::ErrorLayer<BaseSubscriber>, BaseSubscriber>;
#[cfg(not(feature = "tracing"))]
type PreFmtSubscriber =
    Layered<EnvFilter, Layered<Option<Box<dyn Layer<Registry> + Send + Sync>>, Registry>>;

/// A boxed [`Layer`] that can be used with [`LogPlugin::fmt_layer`].
type BoxedFmtLayer = Box<dyn Layer<PreFmtSubscriber> + Send + Sync + 'static>;

/// Target prefix of the engine crates, logged at [`LogPlugin::level`].
const ENGINE_TARGET: &str = "wde";

/// Overrides for dependencies that are noisy at `INFO`.
const DEFAULT_FILTER: &str = concat!(
    "wgpu=warn,",
    "wgpu_hal::vulkan::instance=error,",
    "naga=warn,",
    "egui_wgpu=error,",
    "winit=warn,",
    "calloop=warn,",
    "cosmic_text=warn,",
    "notify=warn,",
    "notify_debouncer_full=warn"
);

/// Plugin that configures logging for WaterDropEngine applications.
///
/// # Configuration
///
/// The engine and the [`crates`](LogPlugin::crates) of the application are logged at
/// [`level`](LogPlugin::level), every other dependency at `INFO` at most.
///
/// The `RUST_LOG` environment variable overrides plugin settings and uses [`EnvFilter`] syntax.
/// Set `NO_COLOR=1` to disable colored output (see [no-color.org](https://no-color.org/)).
///
/// # Log Levels
///
/// Available log levels (most to least important):
/// - `error!()` - Critical failures
/// - `warn!()` - Important warnings
/// - `info!()` - General information
/// - `debug!()` - Debug information
/// - `trace!()` - Verbose tracing
pub struct LogPlugin {
    /// Extra [`EnvFilter`] directives, applied after the levels.
    pub filter: String,

    /// Level of the engine and of [`crates`](LogPlugin::crates).
    pub level: LogLevel,

    /// Targets of the application crates (crate names with `-` replaced by `_`, matched as
    /// prefixes), logged at [`level`](LogPlugin::level) like the engine.
    pub crates: Vec<String>,

    /// Optionally add an extra [`Layer`] to the tracing subscriber.
    pub custom_layer: fn(app: &mut App) -> Option<BoxedLayer>,

    /// Override the default [`tracing_subscriber::fmt::Layer`] with a custom one.
    /// For example, you can use [`tracing_subscriber::fmt::Layer::without_time`] to remove the
    /// timestamp from the log output.
    pub fmt_layer: fn(app: &mut App) -> Option<BoxedFmtLayer>,

    /// Path of the file logs are written to. The previous log file, if any, is renamed
    /// alongside it with a `-old` suffix before its extension (e.g. `log.txt` -> `log-old.txt`).
    ///
    /// Defaults to `<temp_dir>/waterdropengine/log.txt`.
    pub log_file: std::path::PathBuf
}
impl Default for LogPlugin {
    fn default() -> Self {
        Self {
            filter: String::new(),
            level: LogLevel::INFO,
            crates: Vec::new(),
            custom_layer: |_| None,
            fmt_layer: |_| None,
            log_file: std::env::temp_dir().join("waterdropengine").join("log.txt")
        }
    }
}
impl LogPlugin {
    /// `DEBUG` in debug builds and `INFO` in release builds, overridden by the `--debug` and
    /// `--trace` arguments.
    pub fn auto_level(mut self) -> Self {
        let args = std::env::args().collect::<Vec<_>>();
        self.level = if args.iter().any(|arg| arg == "--trace") {
            LogLevel::TRACE
        } else if cfg!(debug_assertions) || args.iter().any(|arg| arg == "--debug") {
            LogLevel::DEBUG
        } else {
            LogLevel::INFO
        };
        self
    }

    /// Full [`EnvFilter`] directives built from the plugin settings.
    fn directives(&self) -> String {
        let mut directives = format!(
            "{},{ENGINE_TARGET}={}",
            self.level.min(LogLevel::INFO),
            self.level
        );
        for target in &self.crates {
            directives.push_str(&format!(",{target}={}", self.level));
        }
        directives.push_str(&format!(",{DEFAULT_FILTER}"));
        if !self.filter.is_empty() {
            directives.push_str(&format!(",{}", self.filter));
        }
        directives
    }

    /// Overrides the log level of a crate/module target (e.g. `"bevy_asset"`), regardless of `level`.
    pub fn with_crate_level(mut self, target: &str, level: LogLevel) -> Self {
        if !self.filter.is_empty() {
            self.filter.push(',');
        }
        self.filter.push_str(&format!("{target}={level}"));
        self
    }
}
impl Plugin for LogPlugin {
    fn build(&self, app: &mut App) {
        // Configure the panic hook
        configure_panic_hook();

        // Build the tracing subscriber with the configured layers and filters
        let directives = self.directives();
        let (subscriber, _guard) = configure_subscriber(
            app,
            &directives,
            self.custom_layer,
            self.fmt_layer,
            &self.log_file
        );
        app.insert_resource(LoggerGuard(_guard));

        // Set the global logger and subscriber based on the different features enabled
        let logger_already_set = LogTracer::init().is_err();
        let subscriber_already_set = tracing::subscriber::set_global_default(subscriber).is_err();

        // Initial log message
        info!("Starting WaterDropEngine.");
        info!("Logs will be written to {}.", self.log_file.display());

        // Log errors if we failed to set the global logger or subscriber, likely due to another logger/subscriber already being set.
        match (logger_already_set, subscriber_already_set) {
            (true, true) => error!(
                "Could not set global logger and tracing subscriber as they are already set. Consider disabling LogPlugin."
            ),
            (true, false) => error!(
                "Could not set global logger as it is already set. Consider disabling LogPlugin."
            ),
            (false, true) => error!(
                "Could not set global tracing subscriber as it is already set. Consider disabling LogPlugin."
            ),
            (false, false) => ()
        }
        debug!("Log filter: {directives}.");

        // Log warnings about features that may increase memory usage and potential conflicts with existing loggers/subscribers
        #[cfg(feature = "tracing")]
        warn!(
            "Tracing with Tracy is active, memory consumption will grow until a client is connected."
        );
        #[cfg(feature = "puffin")]
        debug!("Tracing with Puffin is active.");
    }
}

fn configure_panic_hook() {
    let custom_hook = panic_handler::PanicHook::get();
    std::panic::set_hook(Box::new(move |infos| {
        #[cfg(feature = "tracing")]
        eprintln!("{}", tracing_error::SpanTrace::capture());
        custom_hook(infos);
    }));
}

fn configure_subscriber(
    app: &mut App,
    directives: &str,
    custom_layer: fn(app: &mut App) -> Option<BoxedLayer>,
    fmt_layer: fn(app: &mut App) -> Option<BoxedFmtLayer>,
    log_file: &std::path::Path
) -> (
    impl tracing::Subscriber + Send + Sync,
    tracing_appender::non_blocking::WorkerGuard
) {
    let subscriber = Registry::default();

    // Add optional layer provided by user
    let subscriber = subscriber.with((custom_layer)(app));

    // Set up filter corresponding to the env variable RUST_LOG, or use the default filter if the env variable is not set or invalid
    let default_filter = directives;
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|from_env_error| {
            _ = from_env_error
                .source()
                .and_then(|source| source.downcast_ref::<ParseError>())
                .map(|parse_err| {
                    // We cannot use the `error!` macro here because the logger is not ready yet.
                    eprintln!("Invalid RUST_LOG environment variable: {parse_err}. Falling back to default filter: {default_filter}");
                });
            Ok::<EnvFilter, FromEnvError>(EnvFilter::builder().parse_lossy(default_filter))
        })
        .unwrap();
    let subscriber = subscriber.with(filter_layer);

    // Register default error layer for capturing backtraces if the `tracing` feature is enabled.
    #[cfg(feature = "tracing")]
    let subscriber = subscriber.with(tracing_error::ErrorLayer::default());

    // Set up writer layer
    let fmt_layer = {
        // By default, use the default fmt layer with stderr as the output
        let fmt_layer = (fmt_layer)(app).unwrap_or_else(|| {
            Box::new(tracing_subscriber::fmt::Layer::default().with_writer(std::io::stderr))
        });

        // Ignore some logs when using tracy
        #[cfg(feature = "tracing")]
        let fmt_layer = fmt_layer.with_filter(tracing_subscriber::filter::FilterFn::new(|meta| {
            meta.fields().field("tracy.frame_mark").is_none()
        }));
        fmt_layer
    };
    let subscriber = subscriber.with(fmt_layer);

    // Keep an in-memory ring buffer of recent logs for panic reports.
    let subscriber = subscriber.with(panic_report_layer::PanicReportLogLayer);

    // Rename old log file
    let path = log_file
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let _ = std::fs::create_dir_all(path);
    let file_name = log_file
        .file_name()
        .unwrap_or_else(|| std::ffi::OsStr::new("log.txt"));
    if log_file.exists() {
        let archived_log_file = {
            let stem = log_file.file_stem().unwrap_or(file_name).to_string_lossy();
            let ext = log_file
                .extension()
                .map(|ext| format!(".{}", ext.to_string_lossy()))
                .unwrap_or_default();
            path.join(format!("{stem}-old{ext}"))
        };
        let _ = std::fs::rename(log_file, &archived_log_file);
    }

    // Set up file appender layer with a non-blocking writer
    let file_appender = tracing_appender::rolling::never(path, file_name);
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    let subscriber = subscriber.with(
        tracing_subscriber::fmt::Layer::default()
            .with_writer(non_blocking)
            .with_ansi(false)
            .with_filter(tracing_subscriber::filter::FilterFn::new(|meta| {
                meta.fields().field("tracy.frame_mark").is_none()
            }))
    );

    // Register editor log layer
    #[cfg(feature = "editor")]
    let subscriber = subscriber.with(editor_layer::EditorLogLayer);

    // Register puffin layer
    #[cfg(feature = "puffin")]
    let subscriber = subscriber.with(PuffinLayer::default());
    #[cfg(feature = "puffin")]
    {
        puffin::set_scopes_on(true);
        app.add_systems(Update, puffin_layer::puffin_new_frame_system);
    }

    // Register tracy layer
    #[cfg(feature = "tracing")]
    let subscriber = subscriber.with(tracing_tracy::TracyLayer::default());

    (subscriber, _guard)
}
