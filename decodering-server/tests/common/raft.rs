use std::{net::TcpListener, time::Duration};

use actix_web::App;
use actix_web::HttpServer;
use actix_web::rt::{net::TcpStream, spawn, task::JoinHandle, time};
use actix_web::web::Data;
use decodering_auth::api_key::ApiKeyMethod;
use decodering_auth::aws::auth::AwsMethod;
use decodering_auth::tpm::auth::TpmMethod;
use decodering_core::auth::registry::AuthRegistry;
use decodering_db::sqlite::SqliteDatabase;
use decodering_raft::Raft;
use decodering_server::routes::config::config_app;

use crate::common::{init_tracing_once, sqlite_raft_storage, test_config};

pub struct Node {
    pub id: u64,
    pub addr: String,
    pub raft: Raft,
    handle: JoinHandle<std::io::Result<()>>,
}

impl Drop for Node {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

pub async fn spawn_node(id: u64) -> Result<Node, Box<dyn std::error::Error + Send + Sync>> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    listener.set_nonblocking(true)?;
    let addr = listener.local_addr()?.to_string();

    let config = test_config();
    init_tracing_once(&config, &addr);

    let mut registry = AuthRegistry::default();
    registry.register(Box::new(ApiKeyMethod::new()));
    registry.register(Box::new(TpmMethod::new()));
    registry.register(Box::new(AwsMethod::new()));

    let (orchestrator, app_data) = sqlite_raft_storage(&config, id, &addr)
        .await?
        .ok_or("sqlite_raft_storage returned None")?;
    let raft = app_data
        .raft
        .as_ref()
        .ok_or("raft not initialized")?
        .raft
        .clone();

    let config_data = Data::new(config);
    let app_data_wrapper = Data::new(app_data);
    let orchestrator_data = Data::new(orchestrator);
    let auth_registry_data = Data::new(registry);

    let server = HttpServer::new(move || {
        App::new()
            .app_data(config_data.clone())
            .app_data(app_data_wrapper.clone())
            .app_data(orchestrator_data.clone())
            .app_data(auth_registry_data.clone())
            .configure(config_app::<SqliteDatabase>)
    })
    .workers(1)
    .listen(listener)?
    .run();

    let handle = spawn(server);

    for _ in 0..100 {
        if TcpStream::connect(&addr).await.is_ok() {
            break;
        }
        time::sleep(Duration::from_millis(20)).await;
    }

    Ok(Node {
        id,
        addr,
        raft,
        handle,
    })
}

pub async fn step_init_raft_addr(
    addr: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let resp = reqwest::Client::new()
        .post(format!("http://{addr}/raft/init"))
        .json(&serde_json::json!({ "raft_init": [] }))
        .send()
        .await?;
    assert_eq!(resp.status(), 200);
    Ok(())
}

pub async fn step_metrics_raft_addr(
    addr: &str,
    node_id: u64,
    leader_id: u64,
    members: &[(u64, &str)],
    nodes: &[(u64, &str)],
    state: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let resp = reqwest::Client::new()
        .post(format!("http://{addr}/raft/metrics"))
        .json(&serde_json::json!([]))
        .send()
        .await?;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await?;

    assert_eq!(body["osl_version"], "1.0.0");
    assert_eq!(body["status"], "raft-metrics");
    assert_eq!(body["message"], "Raft node metrics");

    let data = &body["data"];
    assert_eq!(data["id"], node_id);
    assert_eq!(data["current_term"], 1);
    assert_eq!(data["current_leader"], leader_id);
    assert_eq!(data["state"], state);

    assert!(data["running_state"]["Ok"].is_null());

    assert_eq!(data["vote"]["leader_id"]["term"], 1);
    assert_eq!(data["vote"]["leader_id"]["voted_for"], leader_id);
    assert_eq!(data["vote"]["committed"], true);

    let membership = &data["membership_config"];
    let expected_voters: Vec<u64> = members.iter().map(|(id, _)| *id).collect();
    assert_eq!(
        membership["membership"]["configs"],
        serde_json::json!([expected_voters])
    );
    for (id, addr) in nodes {
        assert_eq!(
            membership["membership"]["nodes"][id.to_string()]["addr"],
            *addr
        );
    }

    if node_id == leader_id {
        for (id, _) in members {
            let key = id.to_string();
            assert!(data["heartbeat"][&key].is_number());
            assert_eq!(data["replication"][&key]["leader_id"], leader_id);
            assert!(data["replication"][&key]["index"].is_number());
        }
    }

    Ok(())
}
