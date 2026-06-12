#![recursion_limit = "256"]

use std::sync::Arc;

use clap::Parser;
use decodering_auth::api_key::ApiKeyMethod;
use decodering_auth::aws::auth::AwsMethod;
use decodering_auth::tpm::auth::TpmMethod;
use decodering_core::auth::registry::AuthRegistry;
use decodering_core::metrics::{Metrics, NoopMetrics};
use decodering_core::plugin::orchestrator::Orchestrator;
use decodering_db::postgres::PostgresDatabase;
use decodering_db::sqlite::SqliteDatabase;
use decodering_server::ServerConfig;
use decodering_server::app_data::AppData;
use decodering_server::config::load_config;
use decodering_server::config::{StorageBackend, StorageConfig};
use decodering_server::logger::init_tracing;
use decodering_server::routes::RouteExtensions;
use decodering_server::run_server;
use dotenvy::dotenv;

#[derive(Parser, Clone, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Opt {
    #[clap(long)]
    pub id: u64,

    #[clap(long)]
    pub addr: String,
}

#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let options = Opt::parse();

    dotenv().ok();
    let config = load_config()?;
    let _guards = init_tracing(&config, &options.addr)?;

    let mut registry = AuthRegistry::default();
    registry.register(Box::new(ApiKeyMethod::new()));
    registry.register(Box::new(TpmMethod::new()));
    registry.register(Box::new(AwsMethod::new()));

    let metrics: Arc<dyn Metrics> = Arc::new(NoopMetrics);
    let route_extensions = RouteExtensions::default();

    let mut orchestrator = Orchestrator::new();
    orchestrator
        .load_wasm_plugins_from_dir(&config.plugin_dir, &metrics)
        .map_err(|e| {
            tracing::error!(error=%e, "Failed to initialize orchestrator");
            e
        })?;

    match &config.storage {
        StorageConfig::Raft {
            log_dir,
            log_prefix,
        } => {
            let raft_log_dir = format!("{}/{}.{}.db", log_dir, log_prefix, options.addr);
            let app = AppData::<SqliteDatabase>::new_raft(
                options.id,
                raft_log_dir,
                &options.addr,
                config.auto_migrate,
            )
            .await?;
            let server_config = ServerConfig {
                config,
                app,
                route_exts: route_extensions,
                orchestrator,
                auth_registry: registry,
                metrics,
                addr: options.addr,
            };
            run_server(server_config).await.map_err(Into::into)
        }
        StorageConfig::Single { database_url } => match config.storage_backend {
            StorageBackend::Postgres => {
                let app = AppData::<PostgresDatabase>::new(
                    database_url,
                    options.addr.clone(),
                    config.auto_migrate,
                )
                .await?;

                let server_config = ServerConfig {
                    config,
                    app,
                    route_exts: route_extensions,
                    orchestrator,
                    auth_registry: registry,
                    metrics,
                    addr: options.addr,
                };
                run_server(server_config).await.map_err(Into::into)
            }
            StorageBackend::Sqlite => {
                let app = AppData::<SqliteDatabase>::new(
                    database_url,
                    options.addr.clone(),
                    config.auto_migrate,
                )
                .await?;

                let server_config = ServerConfig {
                    config,
                    app,
                    route_exts: route_extensions,
                    orchestrator,
                    auth_registry: registry,
                    metrics,
                    addr: options.addr,
                };

                run_server(server_config).await.map_err(Into::into)
            }
        },
    }
}
