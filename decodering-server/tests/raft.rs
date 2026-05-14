use actix_web::web::Data;
use actix_web::{App, test};
use decodering_core::plugin::orchestrator::Orchestrator;
use decodering_db::sqlite::SqliteDatabase;
use decodering_server::app_data::AppData;
use decodering_server::config::{Config, StorageConfig};
use decodering_server::logger::{LogOutput, init_tracing};
use decodering_server::routes::config::config_app;

fn test_config() -> Config {
    Config {
        storage: StorageConfig::Raft {
            log_dir: format!("/tmp/decodering.test.{}", uuid::Uuid::new_v4()),
            log_prefix: "decodering.test".to_owned(),
        },
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

#[actix_web::test]
async fn test_sqlite_init_raft() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = "127.0.0.1:21001";
    let id = 1;
    let config = test_config();
    let _guards = init_tracing(&config, addr)?;

    let mut orchestrator = Orchestrator::new();
    orchestrator
        .load_wasm_plugins_from_dir(&config.plugin_dir)
        .map_err(|e| {
            tracing::error!(error=%e, "Failed to initialize orchestrator");
            e
        })?;

    let app = match &config.storage {
        StorageConfig::Raft {
            log_dir,
            log_prefix,
        } => {
            let raft_log_dir = format!("{log_dir}/{log_prefix}.{addr}.db");
            AppData::<SqliteDatabase>::init_raft(id, raft_log_dir, addr).await?
        }
        StorageConfig::Postgres { .. } => {
            eprintln!("Skipping test: requires Raft storage");
            return Ok(());
        }
    };

    let config_data = Data::new(config.clone());
    let app_data = Data::new(app);
    let orchestrator_data = Data::new(orchestrator);
    let app = test::init_service(
        App::new()
            .app_data(config_data)
            .app_data(app_data)
            .app_data(orchestrator_data)
            .configure(config_app::<SqliteDatabase>),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/raft/init")
        .set_json(serde_json::json!({ "raft_init": []}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    Ok(())
}
