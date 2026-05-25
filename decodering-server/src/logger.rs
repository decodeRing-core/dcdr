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

#[derive(Debug)]
pub struct InvalidLogOutput(String);

impl std::fmt::Display for InvalidLogOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "invalid log output '{}', expected file|stdout|both",
            self.0
        )
    }
}

impl std::error::Error for InvalidLogOutput {}

impl FromStr for LogOutput {
    type Err = InvalidLogOutput;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "file" => Ok(Self::File),
            "stdout" => Ok(Self::Stdout),
            "both" => Ok(Self::Both),
            other => Err(InvalidLogOutput(other.to_owned())),
        }
    }
}

#[derive(Debug)]
pub enum TracingError {
    InvalidFilter(String),
    FileAppender(String),
}

impl std::fmt::Display for TracingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidFilter(s) => write!(f, "invalid tracing filter: {s}"),
            Self::FileAppender(s) => write!(f, "failed to init file appender: {s}"),
        }
    }
}

impl std::error::Error for TracingError {}

// Guards that must be kept alive for the duration of the program.
// When dropped, the non-blocking writers stop flushing.
pub struct TracingGuards {
    _file: Option<WorkerGuard>,
    _stdout: Option<WorkerGuard>,
}

pub fn init_tracing(config: &Config, addr: &str) -> Result<TracingGuards, TracingError> {
    let env_filter = EnvFilter::from_str(&config.tracing_level)
        .map_err(|e| TracingError::InvalidFilter(e.to_string()))?;

    let (file_layer, file_guard) =
        if matches!(config.server_log_output, LogOutput::File | LogOutput::Both) {
            let log_prefix = format!("{}.{}", config.server_log_prefix, addr);
            let file_appender = RollingFileAppender::builder()
                .rotation(Rotation::DAILY)
                .filename_prefix(log_prefix)
                .filename_suffix("log")
                .max_log_files(config.server_log_max_files)
                .build(&config.server_log_dir)
                .map_err(|e| TracingError::FileAppender(e.to_string()))?;
            let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
            let layer = fmt::layer()
                .with_line_number(true)
                .with_file(true)
                .with_ansi(false)
                .with_writer(non_blocking)
                .boxed();
            (Some(layer), Some(guard))
        } else {
            (None, None)
        };

    let (stdout_layer, stdout_guard) = if matches!(
        config.server_log_output,
        LogOutput::Stdout | LogOutput::Both
    ) {
        let (non_blocking, guard) = tracing_appender::non_blocking(std::io::stdout());
        let layer = fmt::layer()
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

    Ok(TracingGuards {
        _file: file_guard,
        _stdout: stdout_guard,
    })
}
