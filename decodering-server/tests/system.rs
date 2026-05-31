#![allow(clippy::indexing_slicing)]
#![allow(clippy::future_not_send)]
#![allow(clippy::expect_used)]

use std::time::Duration;

use rand::seq::IteratorRandom;
use reqwest::Response;

use crate::common::raft::{spawn_node, step_init_raft_addr, step_metrics_raft_addr};
use crate::common::system::{
    init_system_addr, init_system_addr_success, random_shards, status_system_addr,
    status_system_addr_unlocked, unlock_system_addr, unlock_system_addr_success,
};

mod common;

#[actix_web::test]
async fn test_system_init() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
    init_system_addr_success(&n1.addr, 5, 2).await?;
    init_system_addr_already_initialized(&n1.addr, 5, 2).await?;

    Ok(())
}

#[actix_web::test]
async fn test_system_status() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
    init_system_addr_success(&n1.addr, 5, 2).await?;
    status_system_addr_locked(&n1.addr).await?;
    Ok(())
}

#[actix_web::test]
async fn test_system_unlock_empty_shards() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
    init_system_addr_success(&n1.addr, 5, 2).await?;
    unlock_system_addr_failed(&n1.addr, &[]).await?;
    Ok(())
}

#[actix_web::test]
async fn test_system_unlock_incorrect_shards()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
    init_system_addr_success(&n1.addr, 5, 2).await?;
    let shards = random_shards(1);
    unlock_system_addr_failed(&n1.addr, &shards).await?;
    let shards = random_shards(2);
    unlock_system_addr_failed(&n1.addr, &shards).await?;
    let shards = random_shards(3);
    unlock_system_addr_failed(&n1.addr, &shards).await?;
    let shards = random_shards(5);
    unlock_system_addr_failed(&n1.addr, &shards).await?;
    Ok(())
}

#[actix_web::test]
async fn test_system_unlock_success_all_shards()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
    Ok(())
}

#[actix_web::test]
async fn test_system_unlock_success_minimum_shards()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
    let picks: Vec<String> = shards.iter().cloned().sample(&mut rand::rng(), 2);
    unlock_system_addr_success(&n1.addr, &picks).await?;

    status_system_addr_unlocked(&n1.addr).await?;
    Ok(())
}

#[actix_web::test]
async fn test_plugin_config_update() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let n1 = spawn_node(1).await?;

    step_init_raft_addr(&n1.addr).await?;
    n1.raft
        .wait(Some(Duration::from_secs(5)))
        .current_leader(n1.id, "wait for current leader to be applied")
        .await?;
    let members = [(1, n1.addr.as_str())];
    let nodes = [(1, n1.addr.as_str())];
    step_metrics_raft_addr(&n1.addr, 1, 1, &members, &nodes, "Leader").await?;

    // init
    let (token, shards) = init_system_addr_success(&n1.addr, 5, 2).await?;
    let picks: Vec<String> = shards.iter().cloned().sample(&mut rand::rng(), 2);
    unlock_system_addr_success(&n1.addr, &picks).await?;

    // Update plugin config
    update_plugin_config_addr_success(&n1.addr, &token).await?;

    Ok(())
}

#[actix_web::test]
async fn test_plugin_config_update_locked() -> Result<(), Box<dyn std::error::Error + Send + Sync>>
{
    let n1 = spawn_node(1).await?;

    step_init_raft_addr(&n1.addr).await?;
    n1.raft
        .wait(Some(Duration::from_secs(5)))
        .current_leader(n1.id, "wait for current leader to be applied")
        .await?;
    let members = [(1, n1.addr.as_str())];
    let nodes = [(1, n1.addr.as_str())];
    step_metrics_raft_addr(&n1.addr, 1, 1, &members, &nodes, "Leader").await?;

    let token = "";
    // Update plugin config
    update_plugin_config_addr_locked(&n1.addr, token).await?;

    Ok(())
}

pub async fn status_system_addr_locked(
    addr: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let resp = status_system_addr(addr).await?;

    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["osl_version"], "1.0.0");
    assert_eq!(body["status"], "system-locked");
    assert_eq!(body["message"], "System locked");

    Ok(())
}

pub async fn unlock_system_addr_failed(
    addr: &str,
    shards: &[String],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let resp = unlock_system_addr(addr, shards, 500).await?;

    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["osl_version"], "1.0.0");
    assert!(body["error"].is_object());
    assert_eq!(body["error"]["code"], "operation-failed");
    assert!(body["error"]["message"].is_string());
    assert_eq!(body["error"]["detail"], "Failed to unlock system.");

    Ok(())
}

async fn update_plugin_config_addr(
    addr: &str,
    token: &str,
    expected_status: u16,
) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let resp = reqwest::Client::new()
        .post(format!("http://{addr}/system/plugin/config"))
        .bearer_auth(token)
        .json(&serde_json::json!({
            "plugins_credentials": {
                "openbao-rs": {
                    "vault_token": "s.oUBdSUO8RIeCXzgeOHv2mIoL"
                }
            }
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), expected_status);
    Ok(resp)
}

pub async fn init_system_addr_already_initialized(
    addr: &str,
    threshold: u8,
    total_shares: u8,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let resp = init_system_addr(addr, threshold, total_shares, 400).await?;

    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["osl_version"], "1.0.0");
    assert!(body["error"].is_object());
    assert_eq!(body["error"]["code"], "operation-failed");
    assert!(body["error"]["message"].is_string());
    assert_eq!(body["error"]["detail"], "System already initialized.");

    Ok(())
}

pub async fn update_plugin_config_addr_success(
    addr: &str,
    token: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let resp = update_plugin_config_addr(addr, token, 200).await?;
    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["osl_version"], "1.0.0");
    assert_eq!(body["status"], "operation-completed");
    assert!(body["message"].is_string());
    Ok(())
}

pub async fn update_plugin_config_addr_locked(
    addr: &str,
    token: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let resp = update_plugin_config_addr(addr, token, 403).await?;

    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["osl_version"], "1.0.0");
    assert!(body["error"].is_object());
    assert_eq!(body["error"]["code"], "operation-failed");
    assert!(body["error"]["message"].is_string());
    assert_eq!(body["error"]["detail"], "System locked.");

    Ok(())
}
