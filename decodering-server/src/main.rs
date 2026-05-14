#![recursion_limit = "256"]

use actix_cors::Cors;
use actix_web::HttpServer;
use actix_web::middleware::Compress;
use actix_web::web::Data;
use decodering_core::plugin::orchestrator::Orchestrator;
use decodering_core::tx::Database;
use decodering_db::postgres::PostgresDatabase;
use decodering_db::sqlite::SqliteDatabase;
use decodering_server::app_data::AppData;
use decodering_server::config::{StorageConfig, load_config};
use decodering_server::logger::init_tracing;
use decodering_server::middleware::PropagateRequestId;
use decodering_server::routes::config::config_app;
use dotenvy::dotenv;
use tracing_actix_web::TracingLogger;

use clap::Parser;

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
    let _guards = init_tracing(config, &options.addr)?;

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
            let raft_log_dir = format!("{}/{}.{}.db", log_dir, log_prefix, options.addr);
            let app = AppData::<SqliteDatabase>::init_raft(
                options.id,
                raft_log_dir,
                options.addr.clone(),
            )
            .await?;
            run_server(app, orchestrator, options.addr)
                .await
                .map_err(Into::into)
        }
        StorageConfig::Postgres { database_url } => {
            let app = AppData::<PostgresDatabase>::new(database_url, options.addr.clone()).await?;
            run_server(app, orchestrator, options.addr)
                .await
                .map_err(Into::into)
        }
    }
}

#[allow(clippy::future_not_send)]
async fn run_server<D>(
    app: AppData<D>,
    orchestrator: Orchestrator,
    addr: String,
) -> std::io::Result<()>
where
    D: Database + Clone + 'static,
    for<'a> D::Tx<'a>:,
{
    let app_data = Data::new(app);
    let orchestrator_data = Data::new(orchestrator);
    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_methods(vec!["GET", "POST"])
            .supports_credentials();
        actix_web::App::new()
            .app_data(app_data.clone())
            .app_data(orchestrator_data.clone())
            .wrap(cors)
            .configure(config_app::<D>)
            .wrap(Compress::default())
            .wrap(PropagateRequestId)
            .wrap(TracingLogger::default())
    })
    .bind(addr)?
    .run()
    .await
}
