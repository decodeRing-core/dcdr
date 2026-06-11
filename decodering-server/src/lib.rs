use std::sync::Arc;

use crate::app_data::AppData;
use crate::config::Config;
use crate::middleware::{propagate_request_id, track_http};
use crate::routes::RouteExtensions;
use crate::routes::config::config_app;
use actix_cors::Cors;
use actix_web::HttpServer;
use actix_web::middleware::{Compress, from_fn};
use actix_web::web::Data;
use decodering_core::auth::registry::AuthRegistry;
use decodering_core::metrics::Metrics;
use decodering_core::plugin::orchestrator::Orchestrator;
use decodering_core::tx::Database;
use decodering_db::postgres::PostgresDatabase;
use decodering_db::sqlite::SqliteDatabase;
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

pub enum AppTest {
    Sqlite(AppData<SqliteDatabase>),
    Postgres(AppData<PostgresDatabase>),
}

pub struct ServerConfig<D: Database> {
    pub config: Config,
    pub app: AppData<D>,
    pub route_exts: RouteExtensions,
    pub orchestrator: Orchestrator,
    pub auth_registry: AuthRegistry,
    pub metrics: Arc<dyn Metrics>,
    pub addr: String,
}

// pub async fn run_with(
//     route_exts: RouteExtensions,
//     auth_extend: impl FnOnce(&mut AuthRegistry),
//     metrics: Arc<dyn Metrics>,
// ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
//     let options = Opt::parse();

//     dotenv().ok();
//     let config = load_config()?;
//     let _guards = init_tracing(&config, &options.addr)?;

//     let mut orchestrator = Orchestrator::new();
//     orchestrator
//         .load_wasm_plugins_from_dir(&config.plugin_dir)
//         .map_err(|e| {
//             tracing::error!(error=%e, "Failed to initialize orchestrator");
//             e
//         })?;

//     let mut registry = AuthRegistry::default();
//     registry.register(Box::new(ApiKeyMethod::new()));
//     registry.register(Box::new(TpmMethod::new()));
//     registry.register(Box::new(AwsMethod::new()));
//     auth_extend(&mut registry);

//     match &config.storage {
//         StorageConfig::Raft {
//             log_dir,
//             log_prefix,
//         } => {
//             let raft_log_dir = format!("{}/{}.{}.db", log_dir, log_prefix, options.addr);
//             let app = AppData::<SqliteDatabase>::new_raft(
//                 options.id,
//                 raft_log_dir,
//                 &options.addr,
//                 config.auto_migrate,
//             )
//             .await?;
//             run_server(
//                 config,
//                 app,
//                 route_exts,
//                 orchestrator,
//                 registry,
//                 metrics,
//                 options.addr,
//             )
//             .await
//             .map_err(Into::into)
//         }
//         StorageConfig::Single { database_url } => match config.storage_backend {
//             StorageBackend::Postgres => {
//                 let app = AppData::<PostgresDatabase>::new(
//                     database_url,
//                     options.addr.clone(),
//                     config.auto_migrate,
//                 )
//                 .await?;
//                 run_server(
//                     config,
//                     app,
//                     route_exts,
//                     orchestrator,
//                     registry,
//                     metrics,
//                     options.addr,
//                 )
//                 .await
//                 .map_err(Into::into)
//             }
//             StorageBackend::Sqlite => {
//                 let app = AppData::<SqliteDatabase>::new(
//                     database_url,
//                     options.addr.clone(),
//                     config.auto_migrate,
//                 )
//                 .await?;
//                 run_server(
//                     config,
//                     app,
//                     route_exts,
//                     orchestrator,
//                     registry,
//                     metrics,
//                     options.addr,
//                 )
//                 .await
//                 .map_err(Into::into)
//             }
//         },
//     }
// }

pub async fn run_server<D>(server_config: ServerConfig<D>) -> std::io::Result<()>
where
    D: Database + 'static,
    for<'a> D::Tx<'a>:,
{
    let config_data = Data::new(server_config.config.clone());
    let app_data = Data::new(server_config.app);
    let orchestrator_data = Data::new(server_config.orchestrator);
    let auth_registry_data = Data::new(server_config.auth_registry);
    let metrics_data = Data::new(server_config.metrics);
    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_methods(vec!["GET", "POST"])
            .supports_credentials();
        actix_web::App::new()
            .app_data(config_data.clone())
            .app_data(app_data.clone())
            .app_data(orchestrator_data.clone())
            .app_data(auth_registry_data.clone())
            .app_data(metrics_data.clone())
            .wrap(cors)
            .configure(config_app::<D>(server_config.route_exts.clone()))
            .wrap(Compress::default())
            .wrap(from_fn(propagate_request_id))
            .wrap(from_fn(track_http))
            .wrap(TracingLogger::default())
    })
    .bind(server_config.addr)?
    .run()
    .await
}
