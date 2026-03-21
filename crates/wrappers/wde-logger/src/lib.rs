//! Logging functions and configuration for WaterDropEngine.
//!
//! Provides logging macros reexported from [`tracing`](https://docs.rs/tracing).
//! Configure logging behavior using the [`LogPlugin`].
//! 
//! By default, the filter used is:
//! ```text
//! wgpu_hal=warn,wgpu_core=warn,naga=warn
//! ```
//! to ignore verbose logs from wgpu and naga.
//! 
//!! # Features
//!! - `tracing`: Enables integration with [Tracy](https://tracy.nagisa.org/), a real-time, nanosecond resolution, remote telemetry, hybrid frame and sampling profiler
//!   This feature enables the `tracing-tracy` crate and adds a `TracyLayer` to the tracing subscriber.
//!!   Note that this will increase memory usage until a Tracy client is connected.
//!!   See the [tracing-tracy documentation](https://docs.rs/tracing-tracy) for more information.
//!! - `puffin`: Enables integration with [Puffin](https://github.com/EmbarkStudios/puffin).
//!   This feature adds a `PuffinLayer` to the tracing subscriber and marks a new frame every update.
//!! - `editor`: Enables integration with the editor logging system.
//!   This feature adds a `EditorLogLayer` to the tracing subscriber.

mod puffin_layer;
pub mod editor_layer;

extern crate alloc;

use core::error::Error;

mod once;
pub use once::OnceFlag;

// This crate is already using `tracing` as the global allocator when the `tracing` feature is
// enabled, so we don't need to do it again here.
// #[cfg(feature = "tracing")]
// #[global_allocator]
// static GLOBAL: tracy_client::ProfiledAllocator<std::alloc::System> =
//     tracy_client::ProfiledAllocator::new(std::alloc::System, 100);

pub mod prelude {
    pub use crate::LogPlugin;
    pub use tracing::{
        debug, debug_span, error, error_span, info, info_span, trace, trace_span, warn, warn_span,
    };
    pub use crate::{debug_once, error_once, info_once, trace_once, warn_once};
    pub use tracing::event;
    pub use crate::Level;
}

pub use tracing::{
    self, debug, debug_span, error, error_span, info, info_span, trace, trace_span, warn,
    warn_span, Level,
};
pub use tracing_subscriber;

use bevy::prelude::*;
use tracing_log::LogTracer;
use tracing_subscriber::{
    filter::{FromEnvError, ParseError},
    layer::Layered,
    prelude::*,
    registry::Registry,
    EnvFilter, Layer,
};

#[cfg(feature = "puffin")]
use crate::puffin_layer::PuffinLayer;


/// A boxed [`Layer`] that can be used with [`LogPlugin::custom_layer`].
pub type BoxedLayer = Box<dyn Layer<Registry> + Send + Sync + 'static>;

#[cfg(feature = "tracing")]
type BaseSubscriber =
    Layered<EnvFilter, Layered<Option<Box<dyn Layer<Registry> + Send + Sync>>, Registry>>;

#[cfg(feature = "tracing")]
type PreFmtSubscriber = Layered<tracing_error::ErrorLayer<BaseSubscriber>, BaseSubscriber>;

#[cfg(not(feature = "tracing"))]
type PreFmtSubscriber =
    Layered<EnvFilter, Layered<Option<Box<dyn Layer<Registry> + Send + Sync>>, Registry>>;

/// A boxed [`Layer`] that can be used with [`LogPlugin::fmt_layer`].
pub type BoxedFmtLayer = Box<dyn Layer<PreFmtSubscriber> + Send + Sync + 'static>;

/// The default [`LogPlugin`] [`EnvFilter`].
pub const DEFAULT_FILTER: &str = concat!(
    "wgpu_hal=warn,",
    "wgpu_core=warn,",
    "naga=warn,"
);

/// Plugin that configures logging for WaterDropEngine applications.
///
/// # Configuration
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
///
/// # Filtering
///
/// Use module-specific filters: `"warn,wde_renderer=debug,wde_scene::physics=trace"`
/// More specific filters take precedence over general ones.
///
/// # Performance
///
/// Runtime filters have performance overhead. For maximum performance, use
/// [compile-time filters](https://docs.rs/log/#compile-time-filters) in your `Cargo.toml`.
///
/// # Panics
///
/// Do not add this plugin multiple times. It sets up global logging configuration.
pub struct LogPlugin {
    /// Filters logs using the [`EnvFilter`] format
    pub filter: String,

    /// Filters out logs that are "less than" the given level.
    /// This can be further filtered using the `filter` setting.
    pub level: Level,

    /// Optionally add an extra [`Layer`] to the tracing subscriber
    ///
    /// This function is only called once, when the plugin is built.
    ///
    /// Because [`BoxedLayer`] takes a `dyn Layer`, `Vec<Layer>` is also an acceptable return value.
    ///
    /// Access to [`App`] is also provided to allow for communication between the
    /// [`Subscriber`](tracing::Subscriber) and the [`App`].
    ///
    /// Please see the `examples/app/log_layers.rs` for a complete example.
    pub custom_layer: fn(app: &mut App) -> Option<BoxedLayer>,

    /// Override the default [`tracing_subscriber::fmt::Layer`] with a custom one.
    ///
    /// This differs from [`custom_layer`](Self::custom_layer) in that
    /// [`fmt_layer`](Self::fmt_layer) allows you to overwrite the default formatter layer, while
    /// `custom_layer` only allows you to add additional layers (which are unable to modify the
    /// default formatter).
    ///
    /// For example, you can use [`tracing_subscriber::fmt::Layer::without_time`] to remove the
    /// timestamp from the log output.
    ///
    /// Please see the `examples/app/log_layers.rs` for a complete example.
    pub fmt_layer: fn(app: &mut App) -> Option<BoxedFmtLayer>,
}
impl Default for LogPlugin {
    fn default() -> Self {
        Self {
            filter: DEFAULT_FILTER.to_string(),
            level: Level::INFO,
            custom_layer: |_| None,
            fmt_layer: |_| None,
        }
    }
}
impl LogPlugin {
    /// Sets the log level based on debug/release mode.
    ///
    /// In debug mode, the level is set to `TRACE` if the `tracing` feature is enabled,
    /// otherwise it is set to `DEBUG`.
    /// In release mode, the level is set to `INFO`.
    pub fn auto_level(mut self) -> Self {
        #[cfg(debug_assertions)]
        {
            self.level = if cfg!(feature = "tracing") {
                Level::TRACE
            } else {
                Level::DEBUG
            };
        }
        #[cfg(not(debug_assertions))]
        {
            self.level = Level::INFO;
        }
        self
    }
}
impl Plugin for LogPlugin {
    #[expect(clippy::print_stderr, reason = "Allowed during logger setup")]
    fn build(&self, app: &mut App) {
        #[cfg(feature = "tracing")]
        {
            let old_handler = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |infos| {
                eprintln!("{}", tracing_error::SpanTrace::capture());
                old_handler(infos);
            }));
        }

        let finished_subscriber;
        let subscriber = Registry::default();

        // Add optional layer provided by user
        let subscriber = subscriber.with((self.custom_layer)(app));

        let default_filter = { format!("{},{}", self.level, self.filter) };
        let filter_layer = EnvFilter::try_from_default_env()
            .or_else(|from_env_error| {
                _ = from_env_error
                    .source()
                    .and_then(|source| source.downcast_ref::<ParseError>())
                    .map(|parse_err| {
                        // We cannot use the `error!` macro here because the logger is not ready yet.
                        eprintln!("LogPlugin failed to parse filter from env: {parse_err}");
                    });

                Ok::<EnvFilter, FromEnvError>(EnvFilter::builder().parse_lossy(&default_filter))
            })
            .unwrap();
        let subscriber = subscriber.with(filter_layer);

        #[cfg(feature = "tracing")]
        let subscriber = subscriber.with(tracing_error::ErrorLayer::default());

        {
            #[cfg(feature = "tracing")]
            let tracy_layer = tracing_tracy::TracyLayer::default();

            let fmt_layer = (self.fmt_layer)(app).unwrap_or_else(|| {
                // note: the implementation of `Default` reads from the env var NO_COLOR
                // to decide whether to use ANSI color codes, which is common convention
                // https://no-color.org/
                Box::new(tracing_subscriber::fmt::Layer::default().with_writer(std::io::stderr))
            });

            // bevy_render::renderer logs a `tracy.frame_mark` event every frame
            // at Level::INFO. Formatted logs should omit it.
            #[cfg(feature = "tracing")]
            let fmt_layer =
                fmt_layer.with_filter(tracing_subscriber::filter::FilterFn::new(|meta| {
                    meta.fields().field("tracy.frame_mark").is_none()
                }));

            let subscriber = subscriber.with(fmt_layer);

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

            #[cfg(feature = "tracing")]
            let subscriber = subscriber.with(tracy_layer);
            finished_subscriber = subscriber;
        }

        let logger_already_set = LogTracer::init().is_err();
        let subscriber_already_set =
            tracing::subscriber::set_global_default(finished_subscriber).is_err();

        #[cfg(feature = "tracing")]
        warn!("Tracing with Tracy is active, memory consumption will grow until a client is connected");

        #[cfg(feature = "puffin")]
        info!("Tracing with Puffin is active");

        #[cfg(feature = "editor")]
        info!("Tracing with editor log layer is active");

        match (logger_already_set, subscriber_already_set) {
            (true, true) => error!(
                "Could not set global logger and tracing subscriber as they are already set. Consider disabling LogPlugin."
            ),
            (true, false) => error!("Could not set global logger as it is already set. Consider disabling LogPlugin."),
            (false, true) => error!("Could not set global tracing subscriber as it is already set. Consider disabling LogPlugin."),
            (false, false) => (),
        }
    }
}

