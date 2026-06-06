#![allow(dead_code)]
use std::sync::Once;

use decodering_core::plugin::orchestrator::Orchestrator;
use decodering_db::sqlite::SqliteDatabase;
use decodering_server::app_data::AppData;
use decodering_server::config::{Config, StorageBackend, StorageConfig};
use decodering_server::logger::{LogOutput, init_tracing};

pub mod raft;
pub mod system;

pub fn test_config() -> Config {
    Config {
        storage: StorageConfig::Raft {
            log_dir: format!("/tmp/decodering.test.{}", uuid::Uuid::new_v4()),
            log_prefix: "decodering.test".to_owned(),
        },
        storage_backend: StorageBackend::Sqlite,
        auto_migrate: true,
        plugin_dir: "./../plugins".into(),
        server_log_output: LogOutput::Both,
        server_log_dir: "/tmp".to_owned(),
        server_log_prefix: "decodering.test".to_owned(),
        server_log_max_files: 0,
        tracing_level:
            "error,decodering=debug,extism=error,extism_pdk=error,tracing_actix_web=info".to_owned(),
        tpm_trust_dir: "/tmp".to_owned(),
    }
}

#[allow(clippy::print_stderr)]
pub async fn sqlite_raft_storage(
    config: &Config,
    id: u64,
    addr: &str,
) -> Result<Option<(Orchestrator, AppData<SqliteDatabase>)>, Box<dyn std::error::Error + Send + Sync>>
{
    let mut orchestrator = Orchestrator::new();
    orchestrator
        .load_wasm_plugins_from_dir(&config.plugin_dir)
        .map_err(|e| {
            tracing::error!(error=%e, "Failed to initialize orchestrator");
            e
        })?;

    match &config.storage {
        StorageConfig::Raft {
            log_dir,
            log_prefix,
        } => {
            let raft_log_dir = format!("{log_dir}/{log_prefix}.{addr}.db");
            let app =
                AppData::<SqliteDatabase>::new_raft(id, raft_log_dir, addr, config.auto_migrate)
                    .await?;
            Ok(Some((orchestrator, app)))
        }
        StorageConfig::Single { .. } => {
            eprintln!("Skipping test: requires Raft storage");
            Ok(None)
        }
    }
}

static INIT_TRACING: Once = Once::new();

pub fn init_tracing_once(config: &Config, addr: &str) {
    INIT_TRACING.call_once(|| {
        // Note: guards are dropped here, but for tracing-subscriber's
        // global default subscriber, that's usually fine — the subscriber
        // itself lives on. If your guards do something important
        // (e.g., flushing file appenders), see Option 2.
        let _ = init_tracing(config, addr);
    });
}
