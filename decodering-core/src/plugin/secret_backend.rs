use crate::plugin::osl_contract::ReadResponse;

use super::error::PluginError;
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize, Debug)]
pub enum Capability {
    KvRead,
    KvWrite,
    KvDelete,
    KvTaint,
    KvVersioning,
    LeaseIssue,
    LeaseRenew,
    LeaseRevoke,
    SyncManage,
    SyncRun,
    SyncStatus,
    RotationPolicy,
}

pub trait SecretBackend: Send + Sync {
    fn get(&self, secret_name: &str, version: Option<String>) -> Result<ReadResponse, PluginError>;
    fn put(&self, path: &str, data: &Value) -> Result<String, PluginError>;
    fn destroy(&self, path: &str) -> Result<bool, PluginError>;
    fn delete(&self, path: &str) -> Result<bool, PluginError>;
    fn restore(&self, path: &str) -> Result<bool, PluginError>;
    fn capabilities(&self) -> Result<Vec<Capability>, PluginError>;
}
