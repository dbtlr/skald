mod rotate;

use crate::engine::config::paths;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::prelude::*;

pub use rotate::prune_old_logs;

pub struct LogGuard {
    _file_guard: Option<WorkerGuard>,
}

pub fn init(verbosity: u8, quiet: bool) -> LogGuard {
    let stderr_level = if quiet {
        "error"
    } else {
        match verbosity {
            0 => "warn",
            1 => "info",
            2 => "debug",
            _ => "trace",
        }
    };

    let stderr_filter = EnvFilter::new(stderr_level);
    let stderr_layer = fmt::layer()
        .with_writer(std::io::stderr)
        .with_target(false)
        .with_ansi(true)
        .with_filter(stderr_filter);

    let log_dir = paths::log_dir();
    let file_guard = if std::fs::create_dir_all(&log_dir).is_ok() {
        let file_appender = tracing_appender::rolling::daily(&log_dir, "skald");
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

        let file_filter = EnvFilter::new("debug");
        let file_layer = fmt::layer()
            .with_writer(non_blocking)
            .with_target(true)
            .with_ansi(false)
            .json()
            .with_filter(file_filter);

        tracing_subscriber::registry().with(stderr_layer).with(file_layer).init();

        Some(guard)
    } else {
        tracing_subscriber::registry().with(stderr_layer).init();

        None
    };

    // Best-effort prune on startup
    let _ = prune_old_logs(14);

    LogGuard { _file_guard: file_guard }
}
