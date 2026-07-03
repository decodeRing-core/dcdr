#![allow(clippy::indexing_slicing)]
#![allow(clippy::future_not_send)]
#![allow(clippy::expect_used)]
use std::sync::Arc;
use std::time::Duration;

use actix_http::Request;
use actix_web::body::MessageBody;
use actix_web::dev::{Service, ServiceResponse};
use actix_web::web::Data;
use actix_web::{App, test};
use decodering_core::metrics::{Metrics, NoopMetrics};
use decodering_db::sqlite::SqliteDatabase;
use decodering_server::routes::RouteExtensions;
use decodering_server::routes::config::config_app;

use crate::common::raft::{spawn_node, step_init_raft_addr, step_metrics_raft_addr};
use crate::common::{init_tracing_once, sqlite_raft_storage, test_config};

mod common;

#[actix_web::test]
async fn test_raft_cluster() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let n1 = spawn_node(1).await?;
    let n2 = spawn_node(2).await?;
    let n3 = spawn_node(3).await?;

    step_init_raft_addr(&n1.addr).await?;
    n1.raft
        .wait(Some(Duration::from_secs(5)))
        .current_leader(n1.id, "wait for current leader to be applied")
        .await?;

    let members = [(1, n1.addr.as_str())];
    let nodes = [(1, n1.addr.as_str())];
    step_metrics_raft_addr(&n1.addr, 1, 1, &members, &nodes, "Leader").await?;
    step_metrics_fresh_addr(&n2.addr, 2).await?;
    step_metrics_fresh_addr(&n3.addr, 3).await?;

    // Add node 2 to cluster as a learner
    step_add_learner_raft_addr(&n1.addr, n2.id, &n2.addr).await?;
    n2.raft
        .wait(Some(Duration::from_secs(5)))
        .current_leader(n1.id, "wait for current leader to be applied")
        .await?;
    let nodes = [(n1.id, n1.addr.as_str()), (n2.id, n2.addr.as_str())];
    step_metrics_raft_addr(&n2.addr, n2.id, n1.id, &members, &nodes, "Learner").await?;

    // Add node 3 to cluster as a learner
    step_add_learner_raft_addr(&n1.addr, n3.id, &n3.addr).await?;
    n2.raft
        .wait(Some(Duration::from_secs(5)))
        .current_leader(n1.id, "wait for current leader to be applied")
        .await?;
    let nodes = [
        (1, n1.addr.as_str()),
        (2, n2.addr.as_str()),
        (3, n3.addr.as_str()),
    ];
    step_metrics_raft_addr(&n3.addr, n3.id, n1.id, &members, &nodes, "Learner").await?;

    // Change membership
    let payload = "{ \"AddVoterIds\": [1,2, 3] }".to_owned();
    let expected_voters = [1u64, 2u64, 3u64];
    let expected_nodes = [
        (n1.id, n1.addr.as_str()),
        (n2.id, n2.addr.as_str()),
        (n3.id, n3.addr.as_str()),
    ];
    step_change_membership_raft_addr(&n1.addr, payload, &expected_voters, &expected_nodes).await?;

    let nodes = [
        (1, n1.addr.as_str()),
        (2, n2.addr.as_str()),
        (3, n3.addr.as_str()),
    ];
    step_metrics_raft_addr(&n2.addr, n2.id, n1.id, &nodes, &nodes, "Follower").await?;

    let nodes = [
        (1, n1.addr.as_str()),
        (2, n2.addr.as_str()),
        (3, n3.addr.as_str()),
    ];
    step_metrics_raft_addr(&n3.addr, n3.id, n1.id, &nodes, &nodes, "Follower").await?;

    let nodes = [
        (1, n1.addr.as_str()),
        (2, n2.addr.as_str()),
        (3, n3.addr.as_str()),
    ];
    step_metrics_raft_addr(&n1.addr, n1.id, n1.id, &nodes, &nodes, "Leader").await?;
    Ok(())
}

#[actix_web::test]
async fn test_raft_lifecycle() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = "127.0.0.1:21001";
    let id = 1;
    let config = test_config();
    init_tracing_once(&config, addr);

    let (orchestrator, app_data) = sqlite_raft_storage(&config, id, addr)
        .await?
        .ok_or("sqlite_raft_storage returned None")?;

    let Some(raft_bits) = app_data.raft.as_ref() else {
        return Err("raft not initialized".into());
    };
    let raft = raft_bits.raft.clone();

    let metrics: Arc<dyn Metrics> = Arc::new(NoopMetrics);

    let config_data = Data::new(config.clone());
    let app_data_wrapper = Data::new(app_data);
    let orchestrator_data = Data::new(orchestrator);
    let metrics_data = Data::new(metrics);
    let route_exts = RouteExtensions::default();
    let app = test::init_service(
        App::new()
            .app_data(config_data)
            .app_data(app_data_wrapper)
            .app_data(orchestrator_data)
            .app_data(metrics_data)
            .configure(config_app::<SqliteDatabase>(route_exts.clone())),
    )
    .await;

    step_init_raft(&app).await?;
    raft.wait(Some(Duration::from_secs(5)))
        .applied_index(Some(1), "wait for init to be applied")
        .await?;
    step_metrics_raft(&app).await?;

    Ok(())
}

async fn step_add_learner_raft_addr(
    addr: &str,
    id: u64,
    learner_addr: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let resp = reqwest::Client::new()
        .post(format!("http://{addr}/raft/add-learner"))
        .json(&serde_json::json!([id, learner_addr]))
        .send()
        .await?;
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await?;

    // Top-level envelope
    assert_eq!(body["osl_version"], "1.0.0");
    assert_eq!(body["status"], "raft-add-learner");
    assert!(body["message"].is_string());
    assert!(body["data"].is_object());

    // log_id: leader_id present, index is a non-negative integer
    let log_id = &body["data"]["log_id"];
    assert!(log_id["leader_id"].is_u64());
    assert!(log_id["index"].is_u64());

    // data field: present (could be "Noop" or a payload)
    assert!(!body["data"]["data"].is_null());

    // membership.configs: non-empty array of arrays of node ids
    let configs = body["data"]["membership"]["configs"]
        .as_array()
        .expect("configs should be an array");
    assert!(!configs.is_empty(), "configs should not be empty");
    for cfg in configs {
        let group = cfg.as_array().expect("each config should be an array");
        assert!(!group.is_empty(), "each config group should not be empty");
        for node_id in group {
            assert!(node_id.is_u64(), "node ids should be unsigned integers");
        }
    }

    // membership.nodes: non-empty map; each node has a non-empty addr string
    let nodes = body["data"]["membership"]["nodes"]
        .as_object()
        .expect("nodes should be an object");
    assert!(!nodes.is_empty(), "nodes map should not be empty");
    for (id, node) in nodes {
        assert!(
            id.parse::<u64>().is_ok(),
            "node id keys should parse as u64"
        );
        let addr = node["addr"].as_str().expect("node.addr should be a string");
        assert!(!addr.is_empty(), "node.addr should not be empty");
    }

    Ok(())
}

async fn step_change_membership_raft_addr(
    addr: &str,
    payload: String,
    expected_voters: &[u64],
    expected_nodes: &[(u64, &str)],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let resp = reqwest::Client::new()
        .post(format!("http://{addr}/raft/change-membership"))
        .json(&serde_json::from_str::<serde_json::Value>(&payload)?)
        .send()
        .await?;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await?;

    assert_eq!(body["osl_version"], "1.0.0");
    assert_eq!(body["status"], "raft-membership");
    assert_eq!(body["message"], "Raft membership changes");

    let data = &body["data"];
    assert!(data["log_id"]["leader_id"].is_number());
    assert!(data["log_id"]["index"].is_number());
    assert_eq!(data["data"], "Noop");

    let voters: Vec<u64> = expected_voters.to_vec();
    assert_eq!(data["membership"]["configs"], serde_json::json!([voters]));

    let nodes = &data["membership"]["nodes"];
    assert_eq!(
        nodes.as_object().map_or(0, serde_json::Map::len),
        expected_nodes.len()
    );
    for (id, addr) in expected_nodes {
        assert_eq!(nodes[id.to_string()]["addr"], *addr);
    }

    Ok(())
}

async fn step_metrics_fresh_addr(
    addr: &str,
    node_id: u64,
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
    assert_eq!(data["current_term"], 0);
    assert_eq!(data["state"], "Learner");
    assert!(data["current_leader"].is_null());
    assert!(data["running_state"]["Ok"].is_null());

    assert_eq!(data["vote"]["leader_id"]["term"], 0);
    assert_eq!(data["vote"]["leader_id"]["voted_for"], node_id);
    assert_eq!(data["vote"]["committed"], false);

    assert!(data["last_log_index"].is_null());
    assert!(data["committed"].is_null());
    assert!(data["last_applied"].is_null());
    assert!(data["millis_since_quorum_ack"].is_null());
    assert!(data["last_quorum_acked"].is_null());

    let membership = &data["membership_config"];
    assert!(membership["log_id"].is_null());
    assert_eq!(membership["membership"]["configs"], serde_json::json!([]));
    assert_eq!(membership["membership"]["nodes"], serde_json::json!({}));

    assert!(data["heartbeat"].is_null());
    assert!(data["replication"].is_null());

    Ok(())
}

async fn step_init_raft<S, B, E>(app: &S) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    S: actix_web::dev::Service<Request, Response = actix_web::dev::ServiceResponse<B>, Error = E>,
    E: std::fmt::Debug,
{
    let req = test::TestRequest::post()
        .uri("/raft/init")
        .set_json(serde_json::json!({ "raft_init": []}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    Ok(())
}

async fn step_metrics_raft<S, B, E>(app: &S) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    S: Service<Request, Response = ServiceResponse<B>, Error = E>,
    B: MessageBody,
    B::Error: std::fmt::Debug,
    E: std::fmt::Debug,
{
    let req = test::TestRequest::post()
        .uri("/raft/metrics")
        .set_json(serde_json::json!([]))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = test::read_body_json(resp).await;

    // Top-level envelope
    assert_eq!(body["osl_version"], "1.0.0");
    assert_eq!(body["status"], "raft-metrics");
    assert_eq!(body["message"], "Raft node metrics");

    let data = &body["data"];

    // Identity & term
    assert_eq!(data["id"], 1);
    assert_eq!(data["current_term"], 1);
    assert_eq!(data["state"], "Leader");
    assert_eq!(data["current_leader"], 1);

    // running_state should be { "Ok": null }
    assert!(data["running_state"].is_object());
    assert!(data["running_state"]["Ok"].is_null());

    // Vote
    assert_eq!(data["vote"]["leader_id"]["term"], 1);
    assert_eq!(data["vote"]["leader_id"]["voted_for"], 1);
    assert_eq!(data["vote"]["committed"], true);

    // Quorum timing: present but values are runtime-dependent
    assert!(data["millis_since_quorum_ack"].is_number());
    assert!(data["last_quorum_acked"].is_number());

    // Membership config
    let membership = &data["membership_config"];
    assert_eq!(membership["log_id"]["leader_id"], 0);
    assert_eq!(membership["log_id"]["index"], 0);
    assert_eq!(
        membership["membership"]["configs"],
        serde_json::json!([[1]])
    );
    assert_eq!(
        membership["membership"]["nodes"]["1"]["addr"],
        "127.0.0.1:21001"
    );

    // Heartbeat & replication keyed by node id
    assert!(data["heartbeat"]["1"].is_number());
    assert_eq!(data["replication"]["1"]["leader_id"], 1);
    assert!(data["replication"]["1"]["index"].is_number());

    Ok(())
}
