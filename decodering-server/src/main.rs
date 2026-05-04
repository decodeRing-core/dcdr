#![warn(unused_extern_crates)]
#![warn(clippy::cast_lossless)]
#![recursion_limit = "256"]

use actix_cors::Cors;
use actix_web::HttpServer;
use actix_web::middleware::Compress;
use actix_web::web::Data;
use decodering_core::plugin::orchestrator::Orchestrator;
use decodering_core::tx::Database;
use decodering_db::postgres::PostgresDatabase;
use decodering_db::sqlite::SqliteDatabase;
use dotenvy::dotenv;
use tracing_actix_web::TracingLogger;

use crate::app_data::AppData;
use crate::config::{StorageMode, get_config};
use crate::logger::init_tracing;
use crate::middleware::PropagateRequestId;
use crate::routes::config::config_app;
use clap::Parser;

mod app_data;
mod config;
mod error;
mod extractor;
mod handlers;
mod logger;
mod middleware;
mod routes;
mod shamir;

#[derive(Parser, Clone, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Opt {
    #[clap(long)]
    pub id: u64,

    #[clap(long)]
    pub addr: String,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let options = Opt::parse();

    dotenv().ok();
    let config = get_config();
    let _guards = init_tracing(config, &options.addr);

    let raft_log_dir = format!(
        "{}/{}.{}.db",
        config.raft_log_dir, config.raft_log_prefix, options.addr
    );
    tracing::debug!(raft_log_dir, "Raft log directory");

    let mut orchestrator = Orchestrator::new();
    let out = orchestrator.load_wasm_plugins_from_dir(&config.plugin_directory);
    if let Err(error) = out {
        tracing::debug!(error=%error, "Failure to initialize decodering core orchestrator");
        panic!("Failure to initialize decodering core orchestrator");
    };

    match config.storage_mode {
        StorageMode::Raft => {
            let app = AppData::<SqliteDatabase>::init_raft(
                options.id,
                raft_log_dir,
                options.addr.clone(),
            )
            .await?;
            run_server(app, orchestrator, options.addr).await
        }
        StorageMode::Postgres => {
            let app = AppData::<PostgresDatabase>::new(&config.database_url, options.addr.clone())
                .await?;
            run_server(app, orchestrator, options.addr).await
        }
    }
}

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
            //.wrap(Logger::default())
            .wrap(PropagateRequestId)
            .wrap(TracingLogger::default())
    })
    .bind(addr)?
    .run()
    .await
}
