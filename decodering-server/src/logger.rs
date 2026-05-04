use serde::Deserialize;
use std::str::FromStr;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{EnvFilter, Layer, fmt, layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::Config;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogOutput {
    File,
    Stdout,
    Both,
}

impl From<String> for LogOutput {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "file" => LogOutput::File,
            "stdout" => LogOutput::Stdout,
            "both" => LogOutput::Both,
            other => panic!("Invalid log output: {other:?}, expected file|stdout|both"),
        }
    }
}

impl From<&str> for LogOutput {
    fn from(s: &str) -> Self {
        Self::from(s.to_string())
    }
}

/// Guards that must be kept alive for the duration of the program.
/// When dropped, the non-blocking writers stop flushing.
pub struct TracingGuards {
    _file: Option<WorkerGuard>,
    _stdout: Option<WorkerGuard>,
}

pub fn init_tracing(config: &Config, addr: &str) -> TracingGuards {
    let env_filter = EnvFilter::from_str(&config.tracing_level).expect("Invalid tracing level");

    let (file_layer, file_guard) =
        if matches!(config.server_log_output, LogOutput::File | LogOutput::Both) {
            let log_prefix = format!("{}.{}", config.server_log_prefix, addr);
            let file_appender = RollingFileAppender::builder()
                .rotation(Rotation::DAILY)
                .filename_prefix(log_prefix)
                .filename_suffix("log")
                .max_log_files(config.server_log_max_files)
                .build(&config.server_log_dir)
                .expect("initializing rolling file appender failed");

            let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
            let layer = fmt::layer()
                //.compact()
                .with_line_number(true)
                .with_file(true)
                .with_ansi(false)
                .with_writer(non_blocking)
                .boxed();
            (Some(layer), Some(guard))
        } else {
            (None, None)
        };

    // Stdout layer
    let (stdout_layer, stdout_guard) = if matches!(
        config.server_log_output,
        LogOutput::Stdout | LogOutput::Both
    ) {
        let (non_blocking, guard) = tracing_appender::non_blocking(std::io::stdout());
        let layer = fmt::layer()
            //.compact()
            .with_line_number(true)
            .with_file(true)
            .with_writer(non_blocking)
            .boxed();
        (Some(layer), Some(guard))
    } else {
        (None, None)
    };

    tracing_subscriber::registry()
        .with(env_filter)
        .with(file_layer)
        .with(stdout_layer)
        .init();

    TracingGuards {
        _file: file_guard,
        _stdout: stdout_guard,
    }
}
