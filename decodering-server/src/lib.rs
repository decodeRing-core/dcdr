use crate::app_data::AppData;
use crate::config::{Config, StorageBackend, StorageConfig, load_config};
use crate::logger::init_tracing;
use crate::middleware::PropagateRequestId;
use crate::routes::config::config_app;
use actix_cors::Cors;
use actix_web::HttpServer;
use actix_web::middleware::Compress;
use actix_web::web::Data;
use clap::Parser;
use decodering_auth::api_key::ApiKeyMethod;
use decodering_auth::aws::auth::AwsMethod;
use decodering_auth::tpm::auth::TpmMethod;
use decodering_core::auth::registry::AuthRegistry;
use decodering_core::plugin::orchestrator::Orchestrator;
use decodering_core::tx::Database;
use decodering_db::postgres::PostgresDatabase;
use decodering_db::sqlite::SqliteDatabase;
use dotenvy::dotenv;
use tracing_actix_web::TracingLogger;

pub mod app_data;
pub mod audit;
pub mod auth;
pub mod config;
pub mod error;
pub mod extractor;
pub mod handlers;
pub mod logger;
pub mod middleware;
pub mod open_api;
pub mod plugin;
pub mod routes;

#[derive(Parser, Clone, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Opt {
    #[clap(long)]
    pub id: u64,

    #[clap(long)]
    pub addr: String,
}

#[allow(clippy::future_not_send)]
pub async fn run_with(
    extend: impl Fn(&mut actix_web::web::ServiceConfig) + Clone + Send + 'static,
    auth_extend: impl FnOnce(&mut AuthRegistry),
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let options = Opt::parse();

    dotenv().ok();
    let config = load_config()?;
    let _guards = init_tracing(&config, &options.addr)?;

    let mut orchestrator = Orchestrator::new();
    orchestrator
        .load_wasm_plugins_from_dir(&config.plugin_dir)
        .map_err(|e| {
            tracing::error!(error=%e, "Failed to initialize orchestrator");
            e
        })?;

    let mut registry = AuthRegistry::default();
    registry.register(Box::new(ApiKeyMethod::new()));
    registry.register(Box::new(TpmMethod::new()));
    registry.register(Box::new(AwsMethod::new()));
    auth_extend(&mut registry);

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
            run_server(config, app, extend, orchestrator, registry, options.addr)
                .await
                .map_err(Into::into)
        }
        StorageConfig::Single { database_url } => match config.storage_backend {
            StorageBackend::Postgres => {
                let app = AppData::<PostgresDatabase>::new(
                    database_url,
                    options.addr.clone(),
                    config.auto_migrate,
                )
                .await?;
                run_server(config, app, extend, orchestrator, registry, options.addr)
                    .await
                    .map_err(Into::into)
            }
            StorageBackend::Sqlite => {
                let app = AppData::<SqliteDatabase>::new(
                    database_url,
                    options.addr.clone(),
                    config.auto_migrate,
                )
                .await?;
                run_server(config, app, extend, orchestrator, registry, options.addr)
                    .await
                    .map_err(Into::into)
            }
        },
    }
}

#[allow(clippy::future_not_send)]
async fn run_server<D>(
    config: Config,
    app: AppData<D>,
    extend: impl Fn(&mut actix_web::web::ServiceConfig) + Clone + Send + 'static,
    orchestrator: Orchestrator,
    auth_registry: AuthRegistry,
    addr: String,
) -> std::io::Result<()>
where
    D: Database + Clone + 'static,
    for<'a> D::Tx<'a>:,
{
    let config_data = Data::new(config.clone());
    let app_data = Data::new(app);
    let orchestrator_data = Data::new(orchestrator);
    let auth_registry_data = Data::new(auth_registry);
    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_methods(vec!["GET", "POST"])
            .supports_credentials();
        actix_web::App::new()
            .app_data(config_data.clone())
            .app_data(app_data.clone())
            .app_data(orchestrator_data.clone())
            .app_data(auth_registry_data.clone())
            .wrap(cors)
            .configure(config_app::<D>)
            .configure(extend.clone())
            .wrap(Compress::default())
            .wrap(PropagateRequestId)
            .wrap(TracingLogger::default())
    })
    .bind(addr)?
    .run()
    .await
}
