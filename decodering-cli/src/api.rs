use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fmt;

pub type PluginsCredentials = HashMap<String, serde_json::Value>;

#[derive(Serialize)]
pub struct InitRequest {
    pub total_shares: u8,
    pub threshold: u8,
    pub plugins_credentials: PluginsCredentials,
}

#[derive(Serialize, Deserialize)]
pub struct UnlockRequest {
    pub shards: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct RaftInitRequest {
    pub raft_init: Vec<(u64, String)>,
}

#[derive(Deserialize)]
pub struct ApiResponse<T> {
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub message: String,
    pub data: Option<T>,
}

#[derive(Deserialize)]
struct ErrorEnvelope {
    error: ApiError,
}

#[derive(Deserialize, Debug)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub detail: Option<String>,
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let primary = self.detail.as_deref().unwrap_or(&self.message);
        write!(f, "{primary} [{}]", self.code)
    }
}

impl Error for ApiError {}

#[derive(Deserialize)]
pub struct InitResponse {
    pub shards: Vec<String>,
    pub root_token: String,
}

pub async fn init(addr: &str, req: InitRequest) -> Result<InitResponse, Box<dyn Error>> {
    let url = format!("{}/system/init", addr.trim_end_matches('/'));
    let res = reqwest::Client::new().post(url).json(&req).send().await?;
    let resp: ApiResponse<InitResponse> = handle(res).await?;
    resp.data
        .ok_or_else(|| Box::<dyn Error>::from("response missing data"))
}

pub async fn unlock(
    addr: &str,
    req: UnlockRequest,
) -> Result<ApiResponse<serde_json::Value>, Box<dyn Error>> {
    let url = format!("{}/system/unlock", addr.trim_end_matches('/'));
    let res = reqwest::Client::new().post(url).json(&req).send().await?;
    handle(res).await
}

pub async fn status(addr: &str) -> Result<ApiResponse<serde_json::Value>, Box<dyn Error>> {
    let url = format!("{}/system/status", addr.trim_end_matches('/'));
    let res = reqwest::Client::new().get(url).send().await?;
    handle(res).await
}

async fn handle<T: DeserializeOwned>(
    res: reqwest::Response,
) -> Result<ApiResponse<T>, Box<dyn Error>> {
    let status = res.status();
    let body = res.text().await?;

    if status.is_success() {
        return serde_json::from_str(&body)
            .map_err(|e| format!("unexpected response ({status}): {e}: {body}").into());
    }

    match serde_json::from_str::<ErrorEnvelope>(&body) {
        Ok(env) => Err(Box::new(env.error)),
        Err(_) => Err(format!("server returned {status}: {body}").into()),
    }
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
