use std::error::Error;
use std::fmt::Display;
use std::str::FromStr;

use serde::Deserialize;

use crate::logger::LogOutput;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StorageBackend {
    Sqlite,
    Postgres,
}

#[derive(Debug)]
pub struct InvalidStorageBackend(String);

impl Display for InvalidStorageBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "invalid storage backend '{}', expected sqlite|postgres",
            self.0
        )
    }
}

impl Error for InvalidStorageBackend {}

impl FromStr for StorageBackend {
    type Err = InvalidStorageBackend;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "sqlite" => Ok(Self::Sqlite),
            "postgres" => Ok(Self::Postgres),
            other => Err(InvalidStorageBackend(other.to_owned())),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClusterMode {
    Raft,
    Single,
}

#[derive(Debug)]
pub struct InvalidClusterMode(String);

impl Display for InvalidClusterMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid cluster mode '{}', expected single|raft", self.0)
    }
}

impl Error for InvalidClusterMode {}

impl FromStr for ClusterMode {
    type Err = InvalidClusterMode;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "raft" => Ok(Self::Raft),
            "single" => Ok(Self::Single),
            other => Err(InvalidClusterMode(other.to_owned())),
        }
    }
}

#[derive(Debug, Clone)]
pub enum StorageConfig {
    Raft { log_dir: String, log_prefix: String },
    Single { database_url: String },
}

#[derive(Debug, Clone)]
pub struct Config {
    pub plugin_dir: String,
    pub storage: StorageConfig,
    pub storage_backend: StorageBackend,
    pub auto_migrate: bool,
    pub server_log_output: LogOutput,
    pub server_log_dir: String,
    pub server_log_prefix: String,
    pub server_log_max_files: usize,
    pub tracing_level: String,
    pub tpm_trust_dir: String,
}

#[derive(Debug)]
pub struct ConfigError(String);

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "config error: {}", self.0)
    }
}

impl std::error::Error for ConfigError {}

fn env_var(name: &str) -> Result<String, ConfigError> {
    std::env::var(name).map_err(|_| ConfigError(format!("{name} must be set")))
}

fn env_parsed<T: FromStr>(name: &str) -> Result<T, ConfigError>
where
    T::Err: std::fmt::Display,
{
    let raw = env_var(name)?;
    raw.parse().map_err(|e| ConfigError(format!("{name}: {e}")))
}

fn init_config() -> Result<Config, ConfigError> {
    let cluster_mode: ClusterMode = env_parsed("CLUSTER_MODE")?;

    let storage = match cluster_mode {
        ClusterMode::Raft => StorageConfig::Raft {
            log_dir: env_var("RAFT_LOG_DIR")?,
            log_prefix: env_var("RAFT_LOG_PREFIX")?,
        },
        ClusterMode::Single => StorageConfig::Single {
            database_url: env_var("DATABASE_URL")?,
        },
    };

    Ok(Config {
        plugin_dir: env_var("PLUGIN_DIR")?,
        storage,
        storage_backend: env_parsed("STORAGE_BACKEND")?,
        auto_migrate: env_parsed("AUTO_MIGRATE")?,
        server_log_output: env_parsed("SERVER_LOG_OUTPUT")?,
        server_log_dir: env_var("SERVER_LOG_DIR")?,
        server_log_prefix: env_var("SERVER_LOG_PREFIX")?,
        server_log_max_files: env_parsed("SERVER_LOG_MAX_FILES")?,
        tracing_level: env_var("TRACING_LEVEL")?,
        tpm_trust_dir: env_var("TPM_TRUST_DIR")?,
    })
}

pub fn load_config() -> Result<Config, ConfigError> {
    init_config()
}
