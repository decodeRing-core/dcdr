#![allow(clippy::indexing_slicing)]
#![allow(clippy::future_not_send)]
#![allow(clippy::expect_used)]

use std::time::Duration;

use reqwest::Response;

use crate::common::raft::spawn_node;
use crate::common::raft::step_init_raft_addr;
use crate::common::raft::step_metrics_raft_addr;
use crate::common::system::init_system_addr_success;
use crate::common::system::status_system_addr_unlocked;
use crate::common::system::unlock_system_addr_success;

mod common;

#[actix_web::test]
async fn test_create_app() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let n1 = spawn_node(1).await?;

    step_init_raft_addr(&n1.addr).await?;
    n1.raft
        .wait(Some(Duration::from_secs(5)))
        .current_leader(n1.id, "wait for current leader to be applied")
        .await?;
    let members = [(1, n1.addr.as_str())];
    let nodes = [(1, n1.addr.as_str())];
    step_metrics_raft_addr(&n1.addr, 1, 1, &members, &nodes, "Leader").await?;

    // Init system
    let (token, shards) = init_system_addr_success(&n1.addr, 5, 2).await?;
    unlock_system_addr_success(&n1.addr, &shards).await?;
    status_system_addr_unlocked(&n1.addr).await?;

    let _ = create_app_addr_success(&n1.addr, "test-app", &token).await?;
    Ok(())
}

#[actix_web::test]
async fn test_create_app_unauthorized() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let n1 = spawn_node(1).await?;

    step_init_raft_addr(&n1.addr).await?;
    n1.raft
        .wait(Some(Duration::from_secs(5)))
        .current_leader(n1.id, "wait for current leader to be applied")
        .await?;
    let members = [(1, n1.addr.as_str())];
    let nodes = [(1, n1.addr.as_str())];
    step_metrics_raft_addr(&n1.addr, 1, 1, &members, &nodes, "Leader").await?;

    // Init system
    let (_, shards) = init_system_addr_success(&n1.addr, 5, 2).await?;
    unlock_system_addr_success(&n1.addr, &shards).await?;
    status_system_addr_unlocked(&n1.addr).await?;

    create_app_addr_failed(&n1.addr, "test-app", "").await?;
    Ok(())
}

#[actix_web::test]
async fn test_create_app_user_api_key() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let n1 = spawn_node(1).await?;

    step_init_raft_addr(&n1.addr).await?;
    n1.raft
        .wait(Some(Duration::from_secs(5)))
        .current_leader(n1.id, "wait for current leader to be applied")
        .await?;
    let members = [(1, n1.addr.as_str())];
    let nodes = [(1, n1.addr.as_str())];
    step_metrics_raft_addr(&n1.addr, 1, 1, &members, &nodes, "Leader").await?;

    // Init system
    let (token, shards) = init_system_addr_success(&n1.addr, 5, 2).await?;
    unlock_system_addr_success(&n1.addr, &shards).await?;
    status_system_addr_unlocked(&n1.addr).await?;

    let _ = create_app_addr_success(&n1.addr, "test-app", &token).await?;
    create_app_user_api_key_addr_success(&n1.addr, "test-api-key", "human", &token).await?;

    Ok(())
}

#[actix_web::test]
async fn test_create_app_grant_app() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let n1 = spawn_node(1).await?;

    step_init_raft_addr(&n1.addr).await?;
    n1.raft
        .wait(Some(Duration::from_secs(5)))
        .current_leader(n1.id, "wait for current leader to be applied")
        .await?;
    let members = [(1, n1.addr.as_str())];
    let nodes = [(1, n1.addr.as_str())];
    step_metrics_raft_addr(&n1.addr, 1, 1, &members, &nodes, "Leader").await?;

    // Init system
    let (token, shards) = init_system_addr_success(&n1.addr, 5, 2).await?;
    unlock_system_addr_success(&n1.addr, &shards).await?;
    status_system_addr_unlocked(&n1.addr).await?;

    let app_id = create_app_addr_success(&n1.addr, "test-app", &token).await?;
    let (_, principal_id) =
        create_app_user_api_key_addr_success(&n1.addr, "test-api-key", "human", &token).await?;

    create_app_grant_addr_success(&n1.addr, &principal_id, vec![&app_id], &token).await?;

    let app_id_2 = create_app_addr_success(&n1.addr, "test-app-2", &token).await?;

    create_app_grant_addr_success(&n1.addr, &principal_id, vec![&app_id, &app_id_2], &token)
        .await?;

    list_apps_addr_success(&n1.addr, &principal_id, vec![&app_id, &app_id_2], &token).await?;
    Ok(())
}

#[actix_web::test]
async fn test_revoke_app_grant_app() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let n1 = spawn_node(1).await?;

    step_init_raft_addr(&n1.addr).await?;
    n1.raft
        .wait(Some(Duration::from_secs(5)))
        .current_leader(n1.id, "wait for current leader to be applied")
        .await?;
    let members = [(1, n1.addr.as_str())];
    let nodes = [(1, n1.addr.as_str())];
    step_metrics_raft_addr(&n1.addr, 1, 1, &members, &nodes, "Leader").await?;

    // Init system
    let (token, shards) = init_system_addr_success(&n1.addr, 5, 2).await?;
    unlock_system_addr_success(&n1.addr, &shards).await?;
    status_system_addr_unlocked(&n1.addr).await?;

    let app_id = create_app_addr_success(&n1.addr, "test-app", &token).await?;
    let (_, principal_id) =
        create_app_user_api_key_addr_success(&n1.addr, "test-api-key", "human", &token).await?;

    create_app_grant_addr_success(&n1.addr, &principal_id, vec![&app_id], &token).await?;

    let app_id_2 = create_app_addr_success(&n1.addr, "test-app-2", &token).await?;

    create_app_grant_addr_success(&n1.addr, &principal_id, vec![&app_id, &app_id_2], &token)
        .await?;

    list_apps_addr_success(&n1.addr, &principal_id, vec![&app_id, &app_id_2], &token).await?;

    revoke_app_grant_addr_success(&n1.addr, &principal_id, &app_id_2, &token).await?;

    list_apps_addr_success(&n1.addr, &principal_id, vec![&app_id], &token).await?;

    create_app_grant_addr_success(&n1.addr, &principal_id, vec![&app_id, &app_id_2], &token)
        .await?;

    revoke_app_grant_addr_success(&n1.addr, &principal_id, &app_id, &token).await?;

    list_apps_addr_success(&n1.addr, &principal_id, vec![&app_id_2], &token).await?;

    Ok(())
}

#[actix_web::test]
async fn test_create_auth_challenge() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let n1 = spawn_node(1).await?;

    step_init_raft_addr(&n1.addr).await?;
    n1.raft
        .wait(Some(Duration::from_secs(5)))
        .current_leader(n1.id, "wait for current leader to be applied")
        .await?;
    let members = [(1, n1.addr.as_str())];
    let nodes = [(1, n1.addr.as_str())];
    step_metrics_raft_addr(&n1.addr, 1, 1, &members, &nodes, "Leader").await?;

    // Init system
    let (token, shards) = init_system_addr_success(&n1.addr, 5, 2).await?;
    unlock_system_addr_success(&n1.addr, &shards).await?;
    status_system_addr_unlocked(&n1.addr).await?;

    create_auth_challenge_addr_success(&n1.addr, &token).await?;
    Ok(())
}

pub async fn create_app_addr(
    addr: &str,
    app_name: &str,
    token: &str,
    expected_status: u16,
) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let resp = reqwest::Client::new()
        .post(format!("http://{addr}/app/create"))
        .bearer_auth(token)
        .json(&serde_json::json!({
              "app_name": app_name,
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), expected_status);
    Ok(resp)
}

pub async fn create_app_addr_success(
    addr: &str,
    app_name: &str,
    token: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let resp = create_app_addr(addr, app_name, token, 200).await?;

    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["osl_version"], "1.0.0");
    assert_eq!(body["status"], "operation-completed");
    assert_eq!(body["message"], "Operation completed");

    assert!(body["data"].is_object());

    assert!(body["data"]["app_id"].is_string());
    assert_eq!(body["data"]["app_name"], app_name);
    let app_id = serde_json::from_value(body["data"]["app_id"].clone())?;
    Ok(app_id)
}

pub async fn create_app_addr_failed(
    addr: &str,
    app_name: &str,
    token: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let resp = create_app_addr(addr, app_name, token, 403).await?;

    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["osl_version"], "1.0.0");
    assert!(body["error"].is_object());
    assert_eq!(body["error"]["code"], "operation-failed");
    assert!(body["error"]["message"].is_string());
    assert_eq!(body["error"]["detail"], "Unauthorized access.");
    Ok(())
}

pub async fn create_app_user_api_key_addr(
    addr: &str,
    app_name: &str,
    kind: &str,
    token: &str,
    expected_status: u16,
) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let resp = reqwest::Client::new()
        .post(format!("http://{addr}/app/user/create"))
        .bearer_auth(token)
        .json(&serde_json::json!({
              "name": app_name,
              "kind": kind,
              "credential_kind": "apiKey",
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), expected_status);
    Ok(resp)
}

pub async fn create_app_user_api_key_addr_success(
    addr: &str,
    app_name: &str,
    kind: &str,
    token: &str,
) -> Result<(String, String), Box<dyn std::error::Error + Send + Sync>> {
    let resp = create_app_user_api_key_addr(addr, app_name, kind, token, 200).await?;

    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["osl_version"], "1.0.0");
    assert_eq!(body["status"], "operation-completed");
    assert_eq!(body["message"], "Operation completed");

    assert!(body["data"].is_object());

    assert!(body["data"]["key"].is_string());
    assert!(body["data"]["principal_id"].is_string());
    let token = serde_json::from_value(body["data"]["key"].clone())?;
    let principal_id = serde_json::from_value(body["data"]["principal_id"].clone())?;
    Ok((token, principal_id))
}

pub async fn create_app_grant_addr(
    addr: &str,
    principal_id: &str,
    apps: Vec<&String>,
    token: &str,
    expected_status: u16,
) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let resp = reqwest::Client::new()
        .post(format!("http://{addr}/app/user/grant"))
        .bearer_auth(token)
        .json(&serde_json::json!({
            "principal_id": principal_id,
            "apps": apps
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), expected_status);
    Ok(resp)
}

pub async fn create_app_grant_addr_success(
    addr: &str,
    principal_id: &str,
    apps: Vec<&String>,
    token: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let resp = create_app_grant_addr(addr, principal_id, apps, token, 200).await?;

    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["osl_version"], "1.0.0");
    assert_eq!(body["status"], "operation-completed");
    assert_eq!(body["message"], "Operation completed");
    Ok(())
}

pub async fn revoke_app_grant_addr(
    addr: &str,
    principal_id: &str,
    app_id: &str,
    token: &str,
    expected_status: u16,
) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let resp = reqwest::Client::new()
        .post(format!("http://{addr}/app/user/revoke"))
        .bearer_auth(token)
        .json(&serde_json::json!({
            "principal_id": principal_id,
            "app_id": app_id
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), expected_status);
    Ok(resp)
}

pub async fn revoke_app_grant_addr_success(
    addr: &str,
    principal_id: &str,
    app_id: &str,
    token: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let resp = revoke_app_grant_addr(addr, principal_id, app_id, token, 200).await?;

    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["osl_version"], "1.0.0");
    assert_eq!(body["status"], "operation-completed");
    assert_eq!(body["message"], "Operation completed");
    Ok(())
}

pub async fn list_apps_addr(
    addr: &str,
    principal_id: &str,
    token: &str,
    expected_status: u16,
) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let resp = reqwest::Client::new()
        .post(format!("http://{addr}/app/user/list"))
        .bearer_auth(token)
        .json(&serde_json::json!({
            "principal_id": principal_id,
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), expected_status);
    Ok(resp)
}

pub async fn list_apps_addr_success(
    addr: &str,
    principal_id: &str,
    expected_apps: Vec<&str>,
    token: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let resp = list_apps_addr(addr, principal_id, token, 200).await?;

    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["osl_version"], "1.0.0");
    assert_eq!(body["status"], "operation-completed");
    assert_eq!(body["message"], "Operation completed");

    assert!(body["data"].is_array());

    let empty = Vec::new();
    let data = body["data"].as_array().unwrap_or(&empty);
    assert_eq!(data.len(), expected_apps.len());
    for app_id in expected_apps {
        assert!(
            data.iter().any(|app| app["app_id"] == app_id),
            "expected app_id {app_id} not found in data: {data:?}"
        );
    }
    Ok(())
}

pub async fn create_auth_challenge_addr(
    addr: &str,
    token: &str,
    expected_status: u16,
) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let resp = reqwest::Client::new()
        .post(format!("http://{addr}/app/user/auth/challenge"))
        .bearer_auth(token)
        .json(&serde_json::json!({
            "credential_kind": "trustedPlatformModule",
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), expected_status);
    Ok(resp)
}

pub async fn create_auth_challenge_addr_success(
    addr: &str,
    token: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let resp = create_auth_challenge_addr(addr, token, 200).await?;

    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["osl_version"], "1.0.0");
    assert_eq!(body["status"], "operation-completed");
    assert_eq!(body["message"], "Operation completed");

    assert!(body["data"].is_object());

    assert!(body["data"]["challenge_id"].is_string());
    assert!(body["data"]["nonce"].is_string());
    assert!(body["data"]["expires_at"].is_i64());

    // Check expires_at is within ~1 hour from now
    let now = i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs(),
    )?;
    let expires_at = body["data"]["expires_at"].as_i64().unwrap_or(0);
    let ttl = expires_at - now;
    assert!(
        ttl > 0 && ttl <= 3600,
        "expires_at TTL out of range: {ttl}s (expires_at={expires_at}, now={now})"
    );
    Ok(())
}
