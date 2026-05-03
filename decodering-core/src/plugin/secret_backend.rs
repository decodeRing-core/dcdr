use super::error::PluginError;
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize, Debug)]
pub struct ReadResponse {
    pub version: u64,
    pub data: Value,
}

pub trait SecretBackend: Send + Sync {
    fn get(&self, secret_name: &str, version: Option<u64>) -> Result<ReadResponse, PluginError>;
    fn put(&self, path: &str, data: &Value) -> Result<u64, PluginError>;
}
