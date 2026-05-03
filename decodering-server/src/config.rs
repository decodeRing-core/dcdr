use std::sync::OnceLock;

use serde::Deserialize;

use crate::logger::LogOutput;

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum StorageMode {
    Raft,
    Postgres,
}

impl From<String> for StorageMode {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "raft" => StorageMode::Raft,
            "postgres" => StorageMode::Postgres,
            other => panic!("Invalid storage mode: {other:?}, expected raft|postgres"),
        }
    }
}

impl From<&str> for StorageMode {
    fn from(s: &str) -> Self {
        Self::from(s.to_string())
    }
}

fn get_env_var(var_name: &str) -> String {
    std::env::var(var_name).unwrap_or_else(|_| panic!("{var_name} must be set"))
}

#[derive(Debug, Clone)]
pub(crate) struct Config {
    pub plugin_directory: String,
    pub database_url: String,
    pub storage_mode: StorageMode,
    pub server_log_output: LogOutput,
    pub server_log_dir: String,
    pub server_log_prefix: String,
    pub server_log_max_files: usize,
    pub raft_log_dir: String,
    pub raft_log_prefix: String,
    pub tracing_level: String,
}

fn init_config() -> Config {
    let plugin_directory = get_env_var("PLUGIN_DIRECTORY");
    let storage_mode = get_env_var("STORAGE_MODE").into();
    let mut database_url = "".to_string();
    let mut raft_log_dir = "".to_string();
    let mut raft_log_prefix = "".to_string();
    if storage_mode == StorageMode::Postgres {
        database_url = get_env_var("DATABASE_URL");
    }
    let server_log_output = get_env_var("SERVER_LOG_OUTPUT").into();
    let server_log_dir = get_env_var("SERVER_LOG_DIR");
    let server_log_prefix = get_env_var("SERVER_LOG_PREFIX");
    let server_log_max_files = get_env_var("SERVER_LOG_MAX_FILES")
        .parse::<usize>()
        .unwrap_or(0);
    if storage_mode == StorageMode::Raft {
        raft_log_dir = get_env_var("RAFT_LOG_DIR");
        raft_log_prefix = get_env_var("RAFT_LOG_PREFIX");
    }
    let tracing_level = get_env_var("TRACING_LEVEL");

    Config {
        plugin_directory,
        database_url,
        storage_mode,
        server_log_output,
        server_log_dir,
        server_log_prefix,
        server_log_max_files,
        raft_log_dir,
        raft_log_prefix,
        tracing_level,
    }
}

static CONFIG: OnceLock<Config> = OnceLock::new();

pub(crate) fn get_config<'a>() -> &'a Config {
    CONFIG.get_or_init(init_config)
}
