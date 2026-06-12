use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;

pub type PluginsCredentials = HashMap<String, serde_json::Value>;

#[derive(Serialize)]
pub struct InitRequest {
    pub total_shares: u8,
    pub threshold: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugins_credentials: Option<PluginsCredentials>,
}

#[derive(Deserialize)]
pub struct InitResponse {
    pub shards: Vec<String>,
    pub root_key: String,
}

pub async fn init(req: InitRequest) -> Result<InitResponse, Box<dyn Error>> {
    // TODO: replace with a real HTTP call to the server, e.g.:
    // let res = client.post(format!("{base}/v1/system/init")).json(&req).send().await?;
    // Ok(res.error_for_status()?.json().await?)
    Err("Not implemented".into())
}
