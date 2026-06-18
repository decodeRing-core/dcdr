use std::collections::HashMap;
use std::error::Error;

use serde::{Deserialize, Serialize};

use crate::api::{ApiResponse, handle, post_auth};

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
pub struct PluginConfigRequest {
    pub plugins_credentials: PluginsCredentials,
}

#[derive(Deserialize)]
pub struct InitResponse {
    pub shards: Vec<String>,
    pub root_token: String,
}

pub async fn system_init(addr: &str, req: InitRequest) -> Result<InitResponse, Box<dyn Error>> {
    let url = format!("{}/system/init", addr.trim_end_matches('/'));
    let res = reqwest::Client::new().post(url).json(&req).send().await?;
    let resp: ApiResponse<InitResponse> = handle(res).await?;
    resp.data
        .ok_or_else(|| Box::<dyn Error>::from("response missing data"))
}

pub async fn system_unlock(
    addr: &str,
    req: UnlockRequest,
) -> Result<ApiResponse<serde_json::Value>, Box<dyn Error>> {
    let url = format!("{}/system/unlock", addr.trim_end_matches('/'));
    let res = reqwest::Client::new().post(url).json(&req).send().await?;
    handle(res).await
}

pub async fn system_status(addr: &str) -> Result<ApiResponse<serde_json::Value>, Box<dyn Error>> {
    let url = format!("{}/system/status", addr.trim_end_matches('/'));
    let res = reqwest::Client::new().get(url).send().await?;
    handle(res).await
}

pub async fn system_plugin_config(
    addr: &str,
    token: &str,
    req: PluginConfigRequest,
) -> Result<ApiResponse<serde_json::Value>, Box<dyn Error>> {
    post_auth(addr, "/system/plugin/config", token, &req).await
}
