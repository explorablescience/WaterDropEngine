use std::borrow::Cow;
use std::fmt::Write;

use backtrace::Backtrace;
use tracing::error;
use uuid::Uuid;

use crate::panic_report_layer;

#[derive(Debug, Clone, serde::Serialize)]
struct Metadata {
    name: Cow<'static, str>,
    version: Cow<'static, str>,
    authors: Cow<'static, str>
}
impl Default for Metadata {
    fn default() -> Self {
        Self {
            name: Cow::Borrowed("WaterDropEngine"),
            version: Cow::Borrowed(env!("CARGO_PKG_VERSION")),
            authors: Cow::Borrowed("ExplorableScience <explorablescience@gmail.com>")
        }
    }
}

pub struct PanicHook;
impl PanicHook {
    pub fn get() -> Box<dyn Fn(&std::panic::PanicHookInfo) + Send + Sync + 'static> {
        Box::new(|infos: &std::panic::PanicHookInfo| {
            let mut report = String::new();
            let panic_message = extract_panic_message(infos);
            if !cfg!(debug_assertions) {
                // Query characteristics of the current build and system to include in the panic report
                let metadata = Metadata::default();
                let system_data = get_system_data();
                let backtrace = render_backtrace();
                let engine_logs = panic_report_layer::panic_report_logs_snapshot();
                let serialized = ReportData::from(metadata, system_data, backtrace, engine_logs, panic_message.clone()).serialize();

                // Write data to disk
                let file_path = match serialized.clone() {
                    Ok(data) => {
                        let panic_reports_dir = std::env::temp_dir().join("waterdropengine");
                        if let Err(e) = std::fs::create_dir_all(&panic_reports_dir) {
                            error!("Failed to create panic reports directory at '{}': {}", panic_reports_dir.display(), e);
                        }
                        let uuid = Uuid::new_v4().hyphenated().to_string();
                        let file_path = std::path::Path::new(&panic_reports_dir).join(format!("panic-report-{uuid}.toml"));
                        match std::fs::write(&file_path, data.as_bytes()) {
                            Ok(_) => Some(file_path),
                            Err(e) => {
                                error!("Failed to write panic report to file '{}': {}", file_path.display(), e);
                                None
                            }
                        }
                    },
                    Err(e) => {
                        error!("Failed to serialize panic report: {}", e);
                        None
                    }
                };

                // Log the panic information using the `error!` macro from `tracing`.
                report.push_str("\nA fatal problem occurred in WaterDropEngine that couldn't be recovered and crashed!");
                    report.push_str(" We are sorry about that.");
                    if let Some(location) = file_path {
                        report.push_str(format!("\n\nWe have generated a panic report at {:#?}.", location).as_str());
                        report.push_str("\nPlease consider sending this report to the developers to help us fix this issue at <explorablescience@gmail.com>.");
                    }
                    let args = std::env::args().collect::<Vec<_>>();
                    if !args.iter().any(|arg| arg == "--debug") {
                        report.push_str("\n\nFor more details, if this happens repeatedly, please run the game with the `--debug` flag to enable debug mode and get a more detailed panic report. This will provide more information about the panic, which can help us identify and fix the underlying issue.");
                    }
            }
            else {
                report.push_str("Panic occurred in WaterDropEngine.\n");
                report.push_str(format!("{}\n", infos.location().unwrap_or_else(|| std::panic::Location::caller())).as_str());
                report.push_str(format!("  {panic_message}").as_str());
            }
            error!("{}", report);
        })
    }
}

fn extract_panic_message(infos: &std::panic::PanicHookInfo) -> String {
    if let Some(message) = infos.payload().downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = infos.payload().downcast_ref::<String>() {
        message.clone()
    } else {
        "<non-string panic payload>".to_string()
    }
}

#[derive(Debug, serde::Serialize)]
struct ReportData {
    name: Cow<'static, str>,
    version: Cow<'static, str>,
    authors: Cow<'static, str>,
    system_data: SystemData,
    backtrace: String,
    engine_logs: Vec<String>,
    panic_message: String
}
impl ReportData {
    fn from(metadata: Metadata, system_data: SystemData, backtrace: String, engine_logs: Vec<String>, panic_message: String) -> Self {
        Self {
            name: metadata.name,
            version: metadata.version,
            authors: metadata.authors,
            system_data,
            backtrace,
            engine_logs,
            panic_message
        }
    }
    fn serialize(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }
}

#[derive(Debug, serde::Serialize)]
struct SystemInfo {
    name: Option<String>,
    kernel_version: Option<String>,
    os_version: Option<String>,
    host_name: Option<String>,
    total_memory: u64, // in bytes
    used_memory: u64, // in bytes
    total_swap: u64, // in bytes
    used_swap: u64, // in bytes
    cpus: CPUInfo
}
#[derive(Debug, serde::Serialize)]

struct CPUInfo {
    count: usize,
    brand: String,
    frequency: u64
}
#[derive(Debug, serde::Serialize)]
struct NetworkInfo {
    name: String,
    total_received: u64, // in bytes
    total_transmitted: u64 // in bytes
}
#[derive(Debug, serde::Serialize)]
struct SystemData {
    system_info: SystemInfo,
    disks: Vec<String>,
    networks: Vec<NetworkInfo>,
    components: Vec<String>
}
fn get_system_data() -> SystemData {
    let mut sys = sysinfo::System::new_all();
    sys.refresh_all();

    let system_info = SystemInfo {
        name: sysinfo::System::name(),
        kernel_version: sysinfo::System::kernel_version(),
        os_version: sysinfo::System::os_version(),
        host_name: sysinfo::System::host_name(),
        total_memory: sys.total_memory(),
        used_memory: sys.used_memory(),
        total_swap: sys.total_swap(),
        used_swap: sys.used_swap(),
        cpus: CPUInfo {
            count: sys.cpus().len(),
            brand: sys.cpus().first().map_or_else(|| "Unknown".into(), |cpu| cpu.brand().to_string()),
            frequency: sys.cpus().first().map_or(0, |cpu| cpu.frequency())
        }
    };
    let disks = sysinfo::Disks::new_with_refreshed_list().iter().map(|disk| format!("{:?}", disk)).collect();
    let networks = sysinfo::Networks::new_with_refreshed_list().iter().map(|(interface_name, ntw_data)| NetworkInfo {
        name: interface_name.clone(),
        total_received: ntw_data.total_received(),
        total_transmitted: ntw_data.total_transmitted()
    }).collect();
    let components = sysinfo::Components::new_with_refreshed_list().iter().map(|component| format!("{:?}", component)).collect();
    SystemData {
        system_info,
        disks,
        networks,
        components
    }
}

fn render_backtrace() -> String {
    // We take padding for address and extra two letters to pad after index.
    #[allow(unused_qualifications)] // needed for pre-1.80 MSRV
    const HEX_WIDTH: usize = std::mem::size_of::<usize>() * 2 + 2;
    // Padding for next lines after frame's address
    const NEXT_SYMBOL_PADDING: usize = HEX_WIDTH + 6;

    let mut backtrace = String::new();

    // Here we iterate over backtrace frames (each corresponds to function's stack)
    // We need to print its address and symbol(e.g. function name), if it is available
    let bt = Backtrace::new();
    let symbols = bt
        .frames()
        .iter()
        .flat_map(|frame| {
            let symbols = frame.symbols();
            if symbols.is_empty() {
                vec![(frame, None, "<unresolved>".to_owned())]
            } else {
                symbols
                    .iter()
                    .map(|s| {
                        (
                            frame,
                            Some(s),
                            s.name()
                                .map(|n| n.to_string())
                                .unwrap_or_else(|| "<unknown>".to_owned()),
                        )
                    })
                    .collect::<Vec<_>>()
            }
        })
        .collect::<Vec<_>>();
    let begin_unwind = "rust_begin_unwind";
    let begin_unwind_start = symbols
        .iter()
        .position(|(_, _, n)| n == begin_unwind)
        .unwrap_or(0);
    for (entry_idx, (frame, symbol, name)) in symbols.iter().skip(begin_unwind_start).enumerate() {
        let ip = frame.ip();
        let _ = writeln!(backtrace, "{entry_idx:4}: {ip:HEX_WIDTH$?} - {name}");
        if let Some(symbol) = symbol {
            // See if there is debug information with file name and line
            if let (Some(file), Some(line)) = (symbol.filename(), symbol.lineno()) {
                let _ = writeln!(
                    backtrace,
                    "{:3$}at {}:{}",
                    "",
                    file.display(),
                    line,
                    NEXT_SYMBOL_PADDING
                );
            }
        }
    }

    backtrace
}

