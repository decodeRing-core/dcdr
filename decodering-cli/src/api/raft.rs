use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::error::Error;

use serde::Deserialize;
use serde::Serialize;

use crate::api::ApiResponse;
use crate::api::handle;

#[derive(Serialize, Deserialize)]
pub struct RaftInitRequest {
    pub raft_init: Vec<(u64, String)>,
}

#[derive(Serialize, Deserialize)]
pub struct Node {
    pub addr: String,
}

#[derive(Serialize, Deserialize)]
pub enum ChangeMembers {
    AddVoterIds(BTreeSet<u64>),
    AddVoters(BTreeMap<u64, Node>),
    RemoveVoters(BTreeSet<u64>),
    ReplaceAllVoters(BTreeSet<u64>),
    AddNodes(BTreeMap<u64, Node>),
    SetNodes(BTreeMap<u64, Node>),
    RemoveNodes(BTreeSet<u64>),
    ReplaceAllNodes(BTreeMap<u64, Node>),
    Batch(Vec<Self>),
}

pub async fn raft_init(
    addr: &str,
    req: RaftInitRequest,
) -> Result<ApiResponse<serde_json::Value>, Box<dyn Error>> {
    let url = format!("{}/raft/init", addr.trim_end_matches('/'));
    let res = reqwest::Client::new().post(url).json(&req).send().await?;
    handle(res).await
}

pub async fn raft_shutdown(addr: &str) -> Result<ApiResponse<serde_json::Value>, Box<dyn Error>> {
    let url = format!("{}/raft/shutdown", addr.trim_end_matches('/'));
    let res = reqwest::Client::new().post(url).send().await?;
    handle(res).await
}

pub async fn raft_add_learner(
    addr: &str,
    node: (u64, String),
) -> Result<ApiResponse<serde_json::Value>, Box<dyn Error>> {
    let url = format!("{}/raft/add-learner", addr.trim_end_matches('/'));
    let res = reqwest::Client::new().post(url).json(&node).send().await?;
    handle(res).await
}

pub async fn raft_change_membership(
    addr: &str,
    change: ChangeMembers,
) -> Result<ApiResponse<serde_json::Value>, Box<dyn Error>> {
    let url = format!("{}/raft/change-membership", addr.trim_end_matches('/'));
    let res = reqwest::Client::new()
        .post(url)
        .json(&change)
        .send()
        .await?;
    handle(res).await
}

pub async fn raft_metrics(addr: &str) -> Result<ApiResponse<serde_json::Value>, Box<dyn Error>> {
    let url = format!("{}/raft/metrics", addr.trim_end_matches('/'));
    let res = reqwest::Client::new().post(url).send().await?;
    handle(res).await
}
